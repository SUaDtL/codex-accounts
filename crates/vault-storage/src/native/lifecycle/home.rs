//! Canonical object identity, not path spelling, selects the lifetime mutex.
use super::super::*;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, marker::PhantomData, rc::Rc, sync::Mutex};
use windows_sys::Win32::System::Threading::{CreateMutexExW, ReleaseMutex, WaitForSingleObject};

static HELD: Mutex<BTreeSet<[u8; 32]>> = Mutex::new(BTreeSet::new());
const MUTEX_RIGHTS: u32 = READ_CONTROL | SYNCHRONIZE | 1;

struct Registration {
    key: [u8; 32],
    remove: bool,
}
impl Registration {
    fn reserve(key: [u8; 32]) -> Result<Self, StorageError> {
        if !HELD
            .lock()
            .map_err(|_| StorageError::RecoveryRequired)?
            .insert(key)
        {
            return Err(StorageError::Busy);
        }
        Ok(Self { key, remove: true })
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        if self.remove {
            if let Ok(mut held) = HELD.lock() {
                held.remove(&self.key);
            }
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct ObjectId {
    volume: u64,
    file: [u8; 16],
}
fn object_id(f: &File) -> Result<ObjectId, StorageError> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: retained disk handle, exact initialized SDK output size.
    if unsafe {
        GetFileInformationByHandleEx(
            f.as_raw_handle(),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            std::mem::size_of_val(&info) as u32,
        )
    } == 0
    {
        return Err(last());
    }
    if info.FileId.Identifier == [0; 16] {
        return Err(StorageError::UnsafePath);
    }
    Ok(ObjectId {
        volume: info.VolumeSerialNumber,
        file: info.FileId.Identifier,
    })
}
fn owner_is_current(f: &File, security: &Security) -> Result<(), StorageError> {
    let mut owner = null_mut();
    let mut sd = null_mut();
    // SAFETY: borrowed READ_CONTROL handle; output owned until SID copy completes.
    let result = unsafe {
        GetSecurityInfo(
            f.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sd,
        )
    };
    let _sd = Allocation(sd);
    if result != 0 {
        return Err(error(result));
    }
    if sid_copy(owner)? != security.user {
        return Err(StorageError::UnsafePath);
    }
    Ok(())
}
fn kernel_descriptor(security: &Security) -> Result<Allocation, StorageError> {
    let mut sid_text = null_mut();
    // SAFETY: copied validated SID, OS-owned output freed by Allocation.
    if unsafe { ConvertSidToStringSidW(security.user.as_ptr().cast_mut().cast(), &mut sid_text) }
        == 0
    {
        return Err(last());
    }
    let sid_text = Allocation(sid_text.cast());
    let mut n = 0;
    while n < 256 && unsafe { *sid_text.0.cast::<u16>().add(n) } != 0 {
        n += 1;
    }
    if n == 256 {
        return Err(StorageError::UnsafePath);
    }
    let sid =
        String::from_utf16(unsafe { std::slice::from_raw_parts(sid_text.0.cast::<u16>(), n) })
            .map_err(|_| StorageError::UnsafePath)?;
    let sddl: Vec<u16> =
        format!("O:{sid}D:P(A;;0x{MUTEX_RIGHTS:08x};;;{sid})(A;;0x{MUTEX_RIGHTS:08x};;;SY)\0")
            .encode_utf16()
            .collect();
    let mut result = null_mut();
    // SAFETY: application-generated descriptor, noninheritable owner/SYSTEM only.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut result,
            null_mut(),
        )
    } == 0
    {
        return Err(last());
    }
    Ok(Allocation(result))
}
fn check_mutex(
    handle: HANDLE,
    expected: &Allocation,
    security: &Security,
) -> Result<(), StorageError> {
    let mut owner = null_mut();
    let mut dacl = null_mut();
    let mut sd = null_mut();
    // SAFETY: READ_CONTROL handle and OS-owned descriptor outputs.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut sd,
        )
    };
    let actual = Allocation(sd);
    if status != 0 {
        return Err(error(status));
    }
    if sid_copy(owner)? != security.user || dacl.is_null() {
        return Err(StorageError::UnsafePath);
    }
    let mut control = 0;
    let mut revision = 0;
    let mut expected_dacl = null_mut();
    let mut present = 0;
    let mut defaulted = 0;
    // SAFETY: both descriptors are produced by Windows; inspect before any slice.
    if unsafe { GetSecurityDescriptorControl(actual.0, &mut control, &mut revision) } == 0
        || control & SE_DACL_PROTECTED == 0
        || unsafe {
            GetSecurityDescriptorDacl(expected.0, &mut present, &mut expected_dacl, &mut defaulted)
        } == 0
        || present == 0
        || expected_dacl.is_null()
        || unsafe { IsValidAcl(dacl) } == 0
        || unsafe { IsValidAcl(expected_dacl) } == 0
    {
        return Err(StorageError::UnsafePath);
    }
    let size = unsafe { (*dacl).AclSize } as usize;
    let expected_size = unsafe { (*expected_dacl).AclSize } as usize;
    if !(8..=1024).contains(&size) || size != expected_size {
        return Err(StorageError::UnsafePath);
    }
    // Exact binary ACL equality also rejects broader grants, inheritance and extra ACEs.
    if unsafe { std::slice::from_raw_parts(dacl.cast::<u8>(), size) }
        != unsafe { std::slice::from_raw_parts(expected_dacl.cast::<u8>(), size) }
    {
        return Err(StorageError::UnsafePath);
    }
    Ok(())
}

pub(super) struct HomeLock {
    directories: Vec<(PathBuf, File, ObjectId)>,
    security: Security,
    mutex: Token,
    registration: Registration,
    abandoned: bool,
    // Windows mutex ownership belongs to the acquiring THREAD, not just process.
    _thread_bound: PhantomData<Rc<()>>,
}
impl std::fmt::Debug for HomeLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HomeLock([REDACTED])")
    }
}
impl HomeLock {
    pub(super) fn acquire(path: &Path) -> Result<Self, StorageError> {
        validate_path(path)?;
        let security = Security::new()?;
        let mut paths: Vec<_> = path.ancestors().collect();
        paths.reverse();
        if paths.len() < 3 || paths.len() > 64 {
            return Err(StorageError::UnsafePath);
        }
        let mut directories = Vec::new();
        for p in paths {
            let file = open_file(
                p,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )?;
            check_object(&file, true)?;
            security.check(&file, false)?;
            let id = object_id(&file)?;
            directories.push((p.to_owned(), file, id));
        }
        let (_, leaf, id) = directories.last().ok_or(StorageError::UnsafePath)?;
        owner_is_current(leaf, &security)?;
        let mut fs = [0u16; 32];
        // SAFETY: exact fixed output buffer and retained volume-backed handle.
        if unsafe {
            GetVolumeInformationByHandleW(
                leaf.as_raw_handle(),
                null_mut(),
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                fs.as_mut_ptr(),
                fs.len() as u32,
            )
        } == 0
        {
            return Err(last());
        }
        if fs[..5] != [78, 84, 70, 83, 0] {
            return Err(StorageError::UnsafePath);
        }
        // Reuse the reviewed sync/path inspection without opening credential files.
        let handles: Vec<File> = directories
            .iter()
            .map(|(_, f, _)| f.try_clone().map_err(|_| StorageError::Io))
            .collect::<Result<_, _>>()?;
        cloud_check(leaf, &handles)?;
        let mut hash = Sha256::new();
        hash.update(b"CodexAccounts/HomeLock/v1\0");
        hash.update((security.user.len() as u32).to_le_bytes());
        hash.update(&security.user);
        hash.update(id.volume.to_le_bytes());
        hash.update(id.file);
        let key: [u8; 32] = hash.finalize().into();
        let registration = Registration::reserve(key)?;
        let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
        let name: Vec<u16> = format!("Global\\CodexAccounts.Home.v1.{hex}\0")
            .encode_utf16()
            .collect();
        let descriptor = kernel_descriptor(&security)?;
        let attrs = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        // SAFETY: generated bounded name/descriptor; no initial recursive ownership.
        let handle = unsafe { CreateMutexExW(&attrs, name.as_ptr(), 0, MUTEX_RIGHTS) };
        if handle.is_null() {
            return Err(last());
        }
        let mutex = Token(handle);
        check_mutex(mutex.0, &descriptor, &security)?;
        // SAFETY: owned mutex handle. Nonblocking acquisition, no stale-marker test.
        let abandoned = match unsafe { WaitForSingleObject(mutex.0, 0) } {
            WAIT_OBJECT_0 => false,
            WAIT_ABANDONED => true,
            WAIT_TIMEOUT => return Err(StorageError::Busy),
            _ => return Err(last()),
        };
        let lock = Self {
            directories,
            security,
            mutex,
            registration,
            abandoned,
            _thread_bound: PhantomData,
        };
        lock.revalidate()?;
        Ok(lock)
    }
    pub(super) fn abandoned(&self) -> bool {
        self.abandoned
    }
    pub(super) fn revalidate(&self) -> Result<(), StorageError> {
        check_mutex(
            self.mutex.0,
            &kernel_descriptor(&self.security)?,
            &self.security,
        )?;
        for (path, file, expected) in &self.directories {
            validate_path(path)?;
            check_object(file, true)?;
            self.security.check(file, false)?;
            let reopened = open_file(
                path,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )?;
            check_object(&reopened, true)?;
            self.security.check(&reopened, false)?;
            if object_id(file)? != *expected || object_id(&reopened)? != *expected {
                return Err(StorageError::ExternalChange);
            }
        }
        let (_, leaf, _) = self.directories.last().ok_or(StorageError::UnsafePath)?;
        owner_is_current(leaf, &self.security)?;
        let handles: Vec<File> = self
            .directories
            .iter()
            .map(|(_, f, _)| f.try_clone().map_err(|_| StorageError::Io))
            .collect::<Result<_, _>>()?;
        cloud_check(leaf, &handles)
    }
}
impl Drop for HomeLock {
    fn drop(&mut self) {
        // SAFETY: !Send/!Sync guard keeps release on the acquiring thread.
        // An unexpected failure poisons this process's reservation, never opens a bypass.
        if unsafe { ReleaseMutex(self.mutex.0) } == 0 {
            self.registration.remove = false;
        }
    }
}
