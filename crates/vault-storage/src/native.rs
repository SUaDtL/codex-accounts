//! Windows x64 only: one scoped unsafe module. See docs/ca-03c-storage.md.
//! No privilege elevation, impersonation, ACL repair, arbitrary-root public API,
//! network share, or unprotected filesystem fallback.
use crate::{
    engine::{Files, Storage},
    records::*,
    StorageError, Vault,
};
use codex_accounts_vault_crypto::{ProtectedRootKey, RootKey};
use std::{
    ffi::{c_void, OsString},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, *},
    Storage::{CloudFilters::*, FileSystem::*},
    System::{
        Com::CoTaskMemFree,
        SystemServices::{
            ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE, SECURITY_SERVICE_ID_BASE_RID,
            SECURITY_TRUSTED_INSTALLER_RID1, SECURITY_TRUSTED_INSTALLER_RID2,
            SECURITY_TRUSTED_INSTALLER_RID3, SECURITY_TRUSTED_INSTALLER_RID4,
            SECURITY_TRUSTED_INSTALLER_RID5,
        },
        Threading::{GetCurrentProcess, OpenProcessToken},
        WindowsProgramming::DRIVE_FIXED,
    },
    UI::Shell::{FOLDERID_LocalAppData, FOLDERID_Profile, SHGetKnownFolderPath},
};

fn error(code: u32) -> StorageError {
    match code {
        ERROR_ACCESS_DENIED => StorageError::AccessDenied,
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => StorageError::Missing,
        ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION => StorageError::Busy,
        ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS => StorageError::AlreadyExists,
        _ => StorageError::Io,
    }
}
fn last() -> StorageError {
    error(unsafe { GetLastError() })
}
struct Allocation(*mut c_void);
impl Drop for Allocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { LocalFree(self.0) };
        }
    }
}
struct Token(HANDLE);
impl Drop for Token {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

// Thread-affine native ownership across both fixed application slots. Only the
// test constructor varies namespace bytes, using its own synthetic fixture.
const LOCATION_RIGHTS: u32 = READ_CONTROL | SYNCHRONIZE | 1;
static LOCATION_HELD: std::sync::Mutex<std::collections::BTreeSet<[u8; 32]>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());
struct LocationReservation {
    key: [u8; 32],
    remove: bool,
}
impl Drop for LocationReservation {
    fn drop(&mut self) {
        if self.remove {
            if let Ok(mut held) = LOCATION_HELD.lock() {
                held.remove(&self.key);
            }
        }
    }
}
struct LocationLock(
    Token,
    LocationReservation,
    std::marker::PhantomData<std::rc::Rc<()>>,
);
impl LocationLock {
    fn acquire(security: &Security, namespace: &[u8]) -> Result<Self, StorageError> {
        use sha2::{Digest, Sha256};
        use windows_sys::Win32::System::Threading::{CreateMutexExW, WaitForSingleObject};
        let mut hash = Sha256::new();
        hash.update(b"CodexAccounts/VaultLocation/v1\0");
        hash.update((security.user.len() as u32).to_le_bytes());
        hash.update(&security.user);
        hash.update(namespace);
        let key: [u8; 32] = hash.finalize().into();
        if !LOCATION_HELD
            .lock()
            .map_err(|_| StorageError::RecoveryRequired)?
            .insert(key)
        {
            return Err(StorageError::Busy);
        }
        let reservation = LocationReservation { key, remove: true };
        let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
        let name: Vec<u16> = format!("Global\\CodexAccounts.VaultLocation.v1.{hex}\0")
            .encode_utf16()
            .collect();
        let descriptor = location_descriptor(security)?;
        let attrs = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        // SAFETY: bounded generated name and owner/SYSTEM descriptor, no initial
        // ownership, no inheritance, no privilege or token changes.
        let handle = unsafe { CreateMutexExW(&attrs, name.as_ptr(), 0, LOCATION_RIGHTS) };
        if handle.is_null() {
            return Err(last());
        }
        let mutex = Token(handle);
        check_location_mutex(mutex.0, &descriptor, security)?;
        // SAFETY: owned nonblocking mutex wait. Windows recursion is separately
        // refused by the process reservation before entering the kernel wait.
        let outcome = unsafe { WaitForSingleObject(mutex.0, 0) };
        if outcome != WAIT_OBJECT_0 && outcome != WAIT_ABANDONED {
            return Err(if outcome == WAIT_TIMEOUT {
                StorageError::Busy
            } else {
                last()
            });
        }
        let owned = Self(mutex, reservation, std::marker::PhantomData);
        if outcome == WAIT_ABANDONED {
            // This wait acquired the abandoned mutex. Dropping our actual owner
            // releases it on the same thread; abandoned state never authorizes IO.
            drop(owned);
            return Err(StorageError::RecoveryRequired);
        }
        Ok(owned)
    }
}
impl Drop for LocationLock {
    fn drop(&mut self) {
        // SAFETY: !Send/!Sync guarantees release on the acquiring thread. An
        // unexpected release failure poisons the process reservation fail-closed.
        if unsafe { windows_sys::Win32::System::Threading::ReleaseMutex(self.0 .0) } == 0 {
            self.1.remove = false;
        }
    }
}
fn location_descriptor(security: &Security) -> Result<Allocation, StorageError> {
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
    let sddl: Vec<u16> = format!(
        "O:{sid}D:P(A;;0x{LOCATION_RIGHTS:08x};;;{sid})(A;;0x{LOCATION_RIGHTS:08x};;;SY)\0"
    )
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
fn check_location_mutex(
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

struct TaskString(*mut u16);
impl Drop for TaskString {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(self.0.cast()) };
    }
}
fn wide(p: &Path) -> Result<Vec<u16>, StorageError> {
    // Reject mapped network/removable/unknown drives before opening any object.
    let Some(Component::Prefix(prefix)) = p.components().next() else {
        return Err(StorageError::UnsafePath);
    };
    let Prefix::Disk(letter) = prefix.kind() else {
        return Err(StorageError::UnsafePath);
    };
    let drive = [u16::from(letter), u16::from(b':'), u16::from(b'\\'), 0];
    // SAFETY: fixed NUL-terminated drive-root string, no implicit current drive.
    if unsafe { GetDriveTypeW(drive.as_ptr()) } != DRIVE_FIXED {
        return Err(StorageError::UnsafePath);
    }
    let mut w: Vec<_> = p.as_os_str().encode_wide().collect();
    if w.is_empty() || w.len() > 240 || w.contains(&0) {
        return Err(StorageError::UnsafePath);
    }
    w.push(0);
    Ok(w)
}
fn validate_path(path: &Path) -> Result<(), StorageError> {
    let mut c = path.components();
    if !matches!(c.next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)))
        || !matches!(c.next(), Some(Component::RootDir))
    {
        return Err(StorageError::UnsafePath);
    }
    for c in c {
        let Component::Normal(s) = c else {
            return Err(StorageError::UnsafePath);
        };
        let s = s.to_str().ok_or(StorageError::UnsafePath)?;
        let n = s.to_ascii_lowercase();
        let stem = n.split('.').next().unwrap_or("");
        if ["con", "prn", "aux", "nul"].contains(&stem)
            || ((stem.starts_with("com") || stem.starts_with("lpt"))
                && stem.len() == 4
                && stem.as_bytes()[3].is_ascii_digit())
        {
            return Err(StorageError::UnsafePath);
        }
        if s.contains([':', '\0'])
            || s.ends_with(['.', ' '])
            || s.chars().any(char::is_control)
            || [
                "onedrive",
                "dropbox",
                "google drive",
                "googledrive",
                "icloud drive",
                "iclouddrive",
                "box",
            ]
            .contains(&n.as_str())
            || n.starts_with("onedrive -")
        {
            return Err(StorageError::UnsafePath);
        }
    }
    if path
        .to_string_lossy()
        .split(['\\', '/'])
        .any(|c| c == "." || c == "..")
    {
        return Err(StorageError::UnsafePath);
    }
    wide(path)?;
    Ok(())
}
const VAULT_CHILD: &str = "CodexAccounts-Vault-v1";
const STORAGE_CONTAINER: &str = "CodexAccounts-Storage";
fn fixed_locations() -> Result<[PathBuf; 2], StorageError> {
    Ok([
        known_folder(&FOLDERID_LocalAppData)?.join(VAULT_CHILD),
        known_folder(&FOLDERID_Profile)?
            .join(STORAGE_CONTAINER)
            .join(VAULT_CHILD),
    ])
}
fn known_folder(id: &windows_sys::core::GUID) -> Result<PathBuf, StorageError> {
    let mut p = null_mut();
    // SAFETY: current user's read-only known-folder query. No create/redirect flag.
    let status = unsafe { SHGetKnownFolderPath(id, 0, null_mut(), &mut p) };
    let p = TaskString(p);
    if status < 0 || p.0.is_null() {
        return Err(StorageError::UnsafePath);
    }
    // SAFETY: OS-owned NUL terminated output, bounded scan, freed by TaskString.
    let mut len = 0;
    while len < 240 && unsafe { *p.0.add(len) } != 0 {
        len += 1;
    }
    if len == 0 || len == 240 {
        return Err(StorageError::UnsafePath);
    }
    let path = PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(p.0, len)
    }));
    validate_path(&path)?;
    Ok(path)
}
struct Security {
    user: Vec<u8>,
    system: Vec<u8>,
    admins: Vec<u8>,
    installer: Vec<u8>,
    descriptor: Allocation,
}
fn sid_copy(sid: *mut c_void) -> Result<Vec<u8>, StorageError> {
    // SAFETY: valid OS descriptor/token owns the SID while it is copied.
    if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
        return Err(StorageError::UnsafePath);
    }
    let len = unsafe { GetLengthSid(sid) } as usize;
    if !(8..=68).contains(&len) {
        return Err(StorageError::UnsafePath);
    }
    Ok(unsafe { std::slice::from_raw_parts(sid.cast::<u8>(), len) }.to_vec())
}
fn known_sid(kind: WELL_KNOWN_SID_TYPE) -> Result<Vec<u8>, StorageError> {
    let mut b = [0u64; 16];
    let mut n = std::mem::size_of_val(&b) as u32;
    // SAFETY: aligned bounded buffer and exact supplied capacity. No token mutation.
    if unsafe { CreateWellKnownSid(kind, null_mut(), b.as_mut_ptr().cast(), &mut n) } == 0 {
        return Err(last());
    }
    sid_copy(b.as_mut_ptr().cast())
}
// Exact Windows servicing principal from SDK SID constants, never a localized
// account-name lookup or an arbitrary service-account whitelist. Ancestors only.
fn installer_sid() -> Result<Vec<u8>, StorageError> {
    let authority = SECURITY_NT_AUTHORITY;
    let mut storage = [0u64; 16];
    let ptr = storage.as_mut_ptr().cast();
    // SAFETY: aligned SID_MAX_SUB_AUTHORITIES-capable buffer, six subauthorities.
    if unsafe { InitializeSid(ptr, &authority, 6) } == 0 {
        return Err(last());
    }
    for (i, value) in [
        SECURITY_SERVICE_ID_BASE_RID as u32,
        SECURITY_TRUSTED_INSTALLER_RID1,
        SECURITY_TRUSTED_INSTALLER_RID2,
        SECURITY_TRUSTED_INSTALLER_RID3,
        SECURITY_TRUSTED_INSTALLER_RID4,
        SECURITY_TRUSTED_INSTALLER_RID5,
    ]
    .into_iter()
    .enumerate()
    {
        // SAFETY: initialized SID has exactly six writable subauthority entries.
        unsafe {
            *GetSidSubAuthority(ptr, i as u32) = value;
        }
    }
    sid_copy(ptr)
}
impl Security {
    fn ancestor_owner_allowed(&self, owner: &[u8]) -> bool {
        owner == self.user
            || owner == self.system
            || owner == self.admins
            || owner == self.installer
    }

    fn new() -> Result<Self, StorageError> {
        let mut t = null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut t) } == 0 {
            return Err(last());
        }
        let t = Token(t);
        let mut b = [0u64; 128];
        let mut n = 0;
        // SAFETY: token query into aligned bounded initialized storage. Size failure refuses.
        if unsafe {
            GetTokenInformation(
                t.0,
                TokenUser,
                b.as_mut_ptr().cast(),
                std::mem::size_of_val(&b) as u32,
                &mut n,
            )
        } == 0
        {
            return Err(last());
        }
        let user = sid_copy(unsafe { (*(b.as_ptr().cast::<TOKEN_USER>())).User.Sid })?;
        let mut s = null_mut();
        if unsafe { ConvertSidToStringSidW(user.as_ptr().cast_mut().cast(), &mut s) } == 0 {
            return Err(last());
        }
        let s = Allocation(s.cast());
        let mut len = 0;
        while len < 256 && unsafe { *(s.0.cast::<u16>().add(len)) } != 0 {
            len += 1;
        }
        if len == 256 {
            return Err(StorageError::UnsafePath);
        }
        let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(s.0.cast::<u16>(), len) })
            .map_err(|_| StorageError::UnsafePath)?;
        let sddl: Vec<u16> = format!("O:{sid}D:P(A;;FA;;;{sid})(A;;FA;;;SY)\0")
            .encode_utf16()
            .collect();
        let mut descriptor = null_mut();
        // SAFETY: descriptor output is owned by Allocation; SDDL is application-generated.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        } == 0
        {
            return Err(last());
        }
        let descriptor = Allocation(descriptor);
        Ok(Self {
            user,
            system: known_sid(WinLocalSystemSid)?,
            admins: known_sid(WinBuiltinAdministratorsSid)?,
            installer: installer_sid()?,
            descriptor,
        })
    }
    fn attrs(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.descriptor.0,
            bInheritHandle: 0,
        }
    }
    fn check(&self, file: &File, strict: bool) -> Result<(), StorageError> {
        let mut owner = null_mut();
        let mut dacl = null_mut();
        let mut sd = null_mut();
        // SAFETY: borrowed READ_CONTROL handle; outputs point within OS-owned descriptor.
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut sd,
            )
        };
        if status != 0 {
            return Err(error(status));
        }
        let sd = Allocation(sd);
        let owner = sid_copy(owner)?;
        if (strict && owner != self.user) || (!strict && !self.ancestor_owner_allowed(&owner)) {
            return Err(StorageError::UnsafePath);
        }
        if dacl.is_null() || unsafe { IsValidAcl(dacl) } == 0 {
            return Err(StorageError::UnsafePath);
        }
        let mut control = 0;
        let mut revision = 0;
        if unsafe { GetSecurityDescriptorControl(sd.0, &mut control, &mut revision) } == 0 {
            return Err(last());
        }
        if strict && control & SE_DACL_PROTECTED == 0 {
            return Err(StorageError::UnsafePath);
        }
        let count = unsafe { (*dacl).AceCount };
        if count > 1024 || (strict && count != 2) {
            return Err(StorageError::UnsafePath);
        }
        let (mut own, mut sys) = (false, false);
        for i in 0..count {
            let mut ace = null_mut();
            if unsafe { GetAce(dacl, i as u32, &mut ace) } == 0 {
                return Err(last());
            }
            if ace.is_null() {
                return Err(StorageError::UnsafePath);
            }
            // SAFETY: GetAce returns a validated ACL entry. Check its type/size before casts.
            let header = unsafe { &*ace.cast::<ACE_HEADER>() };
            if !strict && u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0 {
                continue;
            }
            if u32::from(header.AceType) != ACCESS_ALLOWED_ACE_TYPE {
                if !strict && u32::from(header.AceType) == ACCESS_DENIED_ACE_TYPE {
                    continue;
                }
                return Err(StorageError::UnsafePath);
            }
            if (header.AceSize as usize) < std::mem::size_of::<ACCESS_ALLOWED_ACE>() {
                return Err(StorageError::UnsafePath);
            }
            let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
            let sid_offset = std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart);
            if usize::from(header.AceSize) < sid_offset + 8 {
                return Err(StorageError::UnsafePath);
            }
            let sid_ptr = (&allowed.SidStart as *const u32).cast::<u8>();
            // SAFETY: fixed SID header fits the validated ACE. Check the declared
            // subauthority array fits before asking any SID API to read it.
            let sid_length = 8 + 4 * usize::from(unsafe { *sid_ptr.add(1) });
            if sid_length > 68 || sid_offset + sid_length > usize::from(header.AceSize) {
                return Err(StorageError::UnsafePath);
            }
            let sid = sid_copy(sid_ptr.cast_mut().cast())?;
            if std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart) + sid.len()
                > usize::from(header.AceSize)
            {
                return Err(StorageError::UnsafePath);
            }
            if strict {
                if header.AceFlags != 0 || allowed.Mask != FILE_ALL_ACCESS {
                    return Err(StorageError::UnsafePath);
                }
                if sid == self.user && !own {
                    own = true;
                } else if sid == self.system && !sys {
                    sys = true;
                } else {
                    return Err(StorageError::UnsafePath);
                }
            } else if !self.ancestor_owner_allowed(&sid) {
                // Broad sibling creation alone cannot replace our pinned path.
                // Actual object delete/control/reparse writes remain prohibited.
                let dangerous = DELETE
                    | WRITE_DAC
                    | WRITE_OWNER
                    | FILE_DELETE_CHILD
                    | FILE_WRITE_DATA
                    | FILE_WRITE_ATTRIBUTES
                    | GENERIC_ALL
                    | GENERIC_WRITE;
                if allowed.Mask & dangerous != 0 {
                    return Err(StorageError::UnsafePath);
                }
            }
        }
        if strict && (!own || !sys) {
            return Err(StorageError::UnsafePath);
        }
        Ok(())
    }

    // Distinct metadata classification, never a data-access check. A dangerous
    // grant is tagged only after the ENTIRE supported descriptor has parsed.
    fn observe_ancestor(&self, file: &File) -> Result<AncestorAccess, StorageError> {
        let mut owner = null_mut();
        let mut acl = null_mut();
        let mut sd = null_mut();
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                &mut acl,
                null_mut(),
                &mut sd,
            )
        };
        let sd = Allocation(sd);
        if status != 0 {
            return Err(error(status));
        }
        if unsafe { IsValidSecurityDescriptor(sd.0) } == 0
            || !self.ancestor_owner_allowed(&sid_copy(owner)?)
            || acl.is_null()
            || unsafe { IsValidAcl(acl) } == 0
        {
            return Err(StorageError::UnsafePath);
        }
        let size = unsafe { (*acl).AclSize } as usize;
        let count = unsafe { (*acl).AceCount };
        if !(8..=65536).contains(&size) || count > 1024 {
            return Err(StorageError::UnsafePath);
        }
        let mut unsafe_grant = false;
        for index in 0..count {
            let mut ace = null_mut();
            if unsafe { GetAce(acl, u32::from(index), &mut ace) } == 0 {
                return Err(last());
            }
            let start = ace as usize;
            let base = acl as usize;
            if start < base
                || start
                    .checked_add(std::mem::size_of::<ACE_HEADER>())
                    .is_none_or(|end| end > base + size)
            {
                return Err(StorageError::UnsafePath);
            }
            let header = unsafe { &*ace.cast::<ACE_HEADER>() };
            let ace_size = usize::from(header.AceSize);
            let offset = std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart);
            if start
                .checked_add(ace_size)
                .is_none_or(|end| end > base + size)
                || ace_size < offset + 8
                || header.AceFlags & !0x1f != 0
                || ![ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE]
                    .contains(&u32::from(header.AceType))
            {
                return Err(StorageError::UnsafePath);
            }
            let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
            let sid = unsafe { ace.cast::<u8>().add(offset) };
            let len = 8 + 4 * usize::from(unsafe { *sid.add(1) });
            if len > 68 || offset + len > ace_size {
                return Err(StorageError::UnsafePath);
            }
            let sid = sid_copy(sid.cast())?;
            if u32::from(header.AceType) == ACCESS_ALLOWED_ACE_TYPE
                && u32::from(header.AceFlags) & INHERIT_ONLY_ACE == 0
                && !self.ancestor_owner_allowed(&sid)
            {
                let dangerous = DELETE
                    | WRITE_DAC
                    | WRITE_OWNER
                    | FILE_DELETE_CHILD
                    | FILE_WRITE_DATA
                    | FILE_WRITE_ATTRIBUTES
                    | GENERIC_ALL
                    | GENERIC_WRITE;
                unsafe_grant |= allowed.Mask & dangerous != 0;
            }
        }
        Ok(if unsafe_grant {
            AncestorAccess::DataAccessUnsafe
        } else {
            AncestorAccess::DataAccessSafe
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AncestorAccess {
    DataAccessSafe,
    DataAccessUnsafe,
}
fn descriptor_bytes(file: &File) -> Result<Vec<u8>, StorageError> {
    let mut sd = null_mut();
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sd,
        )
    };
    let sd = Allocation(sd);
    if status != 0 {
        return Err(error(status));
    }
    if unsafe { IsValidSecurityDescriptor(sd.0) } == 0 {
        return Err(StorageError::UnsafePath);
    }
    let len = unsafe { GetSecurityDescriptorLength(sd.0) } as usize;
    if !(1..=65536).contains(&len) {
        return Err(StorageError::UnsafePath);
    }
    Ok(unsafe { std::slice::from_raw_parts(sd.0.cast::<u8>(), len) }.to_vec())
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    volume: u32,
    id: u64,
    size: u64,
    modified: u64,
    attributes: u32,
    links: u32,
}
fn stamp(f: &File) -> Result<Stamp, StorageError> {
    let mut i: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: borrowed disk handle and correctly sized output.
    if unsafe { GetFileInformationByHandle(f.as_raw_handle(), &mut i) } == 0 {
        return Err(last());
    }
    Ok(Stamp {
        volume: i.dwVolumeSerialNumber,
        id: (i.nFileIndexHigh as u64) << 32 | i.nFileIndexLow as u64,
        size: (i.nFileSizeHigh as u64) << 32 | i.nFileSizeLow as u64,
        modified: (i.ftLastWriteTime.dwHighDateTime as u64) << 32
            | i.ftLastWriteTime.dwLowDateTime as u64,
        attributes: i.dwFileAttributes,
        links: i.nNumberOfLinks,
    })
}
fn check_object(f: &File, directory: bool) -> Result<Stamp, StorageError> {
    if unsafe { GetFileType(f.as_raw_handle()) } != FILE_TYPE_DISK {
        return Err(StorageError::UnsafePath);
    }
    let s = stamp(f)?;
    if s.attributes
        & (FILE_ATTRIBUTE_REPARSE_POINT
            | FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
        || (s.attributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
        || (!directory && s.links != 1)
    {
        return Err(StorageError::UnsafePath);
    }
    Ok(s)
}
mod sync;
use sync::cloud_check;

fn open_file(
    path: &Path,
    access: u32,
    share: u32,
    creation: u32,
    security: Option<&Security>,
) -> Result<File, StorageError> {
    let name = wide(path)?;
    let attrs = security.map(Security::attrs);
    // SAFETY: arguments and descriptor stay live for the synchronous call. No
    // inheritance; no recall; leaf reparse is opened itself and then rejected.
    let h = unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            share,
            attrs.as_ref().map_or(null(), |v| v as *const _),
            creation,
            FILE_FLAG_OPEN_REPARSE_POINT
                | FILE_FLAG_BACKUP_SEMANTICS
                | FILE_FLAG_OPEN_NO_RECALL
                | FILE_FLAG_WRITE_THROUGH,
            null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(last());
    }
    Ok(unsafe { File::from_raw_handle(h) })
}
fn read_bytes(f: &mut File) -> Result<Vec<u8>, StorageError> {
    let before = check_object(f, false)?;
    if before.size > MAX_FILE as u64 {
        return Err(StorageError::InputLimit);
    }
    f.seek(SeekFrom::Start(0)).map_err(|_| StorageError::Io)?;
    let mut b = Vec::new();
    f.take(MAX_FILE as u64 + 1)
        .read_to_end(&mut b)
        .map_err(|_| StorageError::Io)?;
    if b.len() as u64 != before.size || stamp(f)? != before {
        return Err(StorageError::ExternalChange);
    }
    Ok(b)
}

/// Retained ancestors deny directory write/delete sharing; exclusive lock handle
/// lasts for the entire Vault session. Lock-file existence alone is never a lock.
pub(crate) struct Disk {
    root: PathBuf,
    dirs: Vec<File>,
    identities: Vec<(u32, u64)>,
    security: Security,
    _lock: File,
    #[cfg(test)]
    location: Option<LocationState>,
    layout: Option<LayoutState>,
    bound_security: Option<Vec<Vec<u8>>>,
    recognized: Option<Vec<RecognizedLeaf>>,
}

struct BoundAncestor {
    path: PathBuf,
    file: File,
    identity: (u32, u64),
    descriptor: Vec<u8>,
    access: AncestorAccess,
}
#[derive(Clone, Copy)]
enum ObservationPolicy {
    StrictAncestors,
    LegacyMetadata,
}
// This object has no Files implementation or child-data opener. Its only
// capability is the fixed directory leaf's checked metadata observation.
struct MetadataWatch {
    path: PathBuf,
    parents: Vec<BoundAncestor>,
    leaf: Option<File>,
    identity: Option<(u32, u64)>,
    policy: ObservationPolicy,
}
impl MetadataWatch {
    fn new(
        path: &Path,
        security: &Security,
        policy: ObservationPolicy,
    ) -> Result<Self, StorageError> {
        validate_path(path)?;
        let mut paths: Vec<_> = path.ancestors().skip(1).collect();
        paths.reverse();
        if !(2..=64).contains(&paths.len()) {
            return Err(StorageError::UnsafePath);
        }
        let mut parents = Vec::new();
        for path in paths {
            let file = open_file(
                path,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )
            .map_err(|e| {
                if e == StorageError::Missing {
                    StorageError::ActiveStateUnknown
                } else {
                    e
                }
            })?;
            let s = check_object(&file, true)?;
            let access = match policy {
                ObservationPolicy::LegacyMetadata => security.observe_ancestor(&file)?,
                ObservationPolicy::StrictAncestors => {
                    security.check(&file, false)?;
                    AncestorAccess::DataAccessSafe
                }
            };
            let descriptor = descriptor_bytes(&file)?;
            parents.push(BoundAncestor {
                path: path.to_owned(),
                file,
                identity: (s.volume, s.id),
                descriptor,
                access,
            });
        }
        let dirs: Vec<_> = parents
            .iter()
            .map(|p| p.file.try_clone().map_err(|_| StorageError::Io))
            .collect::<Result<_, _>>()?;
        check_volume_and_sync(dirs.last().ok_or(StorageError::UnsafePath)?, &dirs)?;
        let leaf = match open_file(
            path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        ) {
            Ok(file) => {
                check_object(&file, true)?;
                Some(file)
            }
            Err(StorageError::Missing) => None,
            Err(e) => return Err(e),
        };
        let identity = leaf
            .as_ref()
            .map(|file| stamp(file).map(|s| (s.volume, s.id)))
            .transpose()?;
        let result = Self {
            path: path.to_owned(),
            parents,
            leaf,
            identity,
            policy,
        };
        result.revalidate(security)?;
        Ok(result)
    }
    fn revalidate(&self, security: &Security) -> Result<(), StorageError> {
        for parent in &self.parents {
            let named = open_file(
                &parent.path,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )?;
            for file in [&parent.file, &named] {
                let stamp = check_object(file, true)?;
                if (stamp.volume, stamp.id) != parent.identity
                    || descriptor_bytes(file)? != parent.descriptor
                {
                    return Err(StorageError::ExternalChange);
                }
                let access = match self.policy {
                    ObservationPolicy::LegacyMetadata => security.observe_ancestor(file)?,
                    ObservationPolicy::StrictAncestors => {
                        security.check(file, false)?;
                        AncestorAccess::DataAccessSafe
                    }
                };
                if access != parent.access {
                    return Err(StorageError::ExternalChange);
                }
            }
        }
        let named = match open_file(
            &self.path,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        ) {
            Ok(file) => {
                let s = check_object(&file, true)?;
                Some((s.volume, s.id))
            }
            Err(StorageError::Missing) => None,
            Err(e) => return Err(e),
        };
        if named != self.identity {
            return Err(StorageError::ExternalChange);
        }
        if let Some(held) = &self.leaf {
            let s = check_object(held, true)?;
            if Some((s.volume, s.id)) != self.identity {
                return Err(StorageError::ExternalChange);
            }
        }
        Ok(())
    }
}
struct LegacyObservation(MetadataWatch);
impl LegacyObservation {
    fn new(path: &Path, security: &Security) -> Result<Self, StorageError> {
        Ok(Self(MetadataWatch::new(
            path,
            security,
            ObservationPolicy::LegacyMetadata,
        )?))
    }
}
struct LayoutState {
    lock: LocationLock,
    legacy: LegacyObservation,
    reserved: MetadataWatch,
    container: MetadataWatch,
    nested: Option<MetadataWatch>,
}
fn directory_names(path: &Path) -> Result<Vec<String>, StorageError> {
    let mut names = Vec::new();
    for item in std::fs::read_dir(path).map_err(io_error)? {
        if names.len() >= MAX_BLOBS * 2 + 3 {
            return Err(StorageError::InputLimit);
        }
        names.push(
            item.map_err(io_error)?
                .file_name()
                .into_string()
                .map_err(|_| StorageError::UnsafePath)?,
        );
    }
    names.sort();
    Ok(names)
}
fn io_error(e: std::io::Error) -> StorageError {
    e.raw_os_error()
        .map_or(StorageError::Io, |code| error(code as u32))
}
impl LayoutState {
    fn revalidate(&self, security: &Security) -> Result<(), StorageError> {
        check_location_mutex(self.lock.0 .0, &location_descriptor(security)?, security)?;
        self.legacy.0.revalidate(security)?;
        self.reserved.revalidate(security)?;
        self.container.revalidate(security)?;
        if let Some(container) = &self.container.leaf {
            security.check(container, true)?;
            if let Some(nested) = &self.nested {
                nested.revalidate(security)?;
                let names = directory_names(&self.container.path)?;
                let expected: Vec<_> = if nested.identity.is_some() {
                    vec![VAULT_CHILD.to_owned()]
                } else {
                    vec![]
                };
                if names != expected {
                    return Err(StorageError::ExternalChange);
                }
            }
        }
        Ok(())
    }
}
struct RecognizedLeaf {
    name: String,
    stamp: Stamp,
    descriptor: Vec<u8>,
    hash: [u8; 32],
}
#[derive(Clone, Copy)]
enum LockAdmission {
    #[cfg(test)]
    HistoricalBootstrap,
    CreateNew,
    ExistingReadOnly,
}

#[cfg(test)]
struct SlotWatch {
    path: PathBuf,
    parents: Vec<(PathBuf, File, (u32, u64))>,
    identity: Option<(u32, u64)>,
}
#[cfg(test)]
impl SlotWatch {
    fn new(path: &Path, security: &Security) -> Result<Self, StorageError> {
        validate_path(path)?;
        let mut paths: Vec<_> = path.ancestors().skip(1).collect();
        paths.reverse();
        if paths.len() < 2 || paths.len() > 64 {
            return Err(StorageError::UnsafePath);
        }
        let mut parents = Vec::new();
        for path in paths {
            let file = open_file(
                path,
                FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )
            .map_err(|e| {
                if e == StorageError::Missing {
                    StorageError::ActiveStateUnknown
                } else {
                    e
                }
            })?;
            let s = check_object(&file, true)?;
            security.check(&file, false)?;
            parents.push((path.to_owned(), file, (s.volume, s.id)));
        }
        let held: Vec<_> = parents
            .iter()
            .map(|(_, f, _)| f.try_clone().map_err(|_| StorageError::Io))
            .collect::<Result<_, _>>()?;
        check_volume_and_sync(held.last().ok_or(StorageError::UnsafePath)?, &held)?;
        let mut watch = Self {
            path: path.to_owned(),
            parents,
            identity: None,
        };
        watch.identity = watch.observe()?;
        Ok(watch)
    }
    fn observe(&self) -> Result<Option<(u32, u64)>, StorageError> {
        // The ancestors were separately validated and retained without delete
        // sharing. Only the final named leaf's genuine Missing means absence.
        let file = match open_file(
            &self.path,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            None,
        ) {
            Ok(file) => file,
            Err(StorageError::Missing) => return Ok(None),
            Err(error) => return Err(error),
        };
        let s = check_object(&file, true)?;
        Ok(Some((s.volume, s.id)))
    }
    fn revalidate(&self, security: &Security) -> Result<(), StorageError> {
        for (path, held, expected) in &self.parents {
            security.check(held, false)?;
            let actual = check_object(held, true)?;
            let reopened = open_file(
                path,
                FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )?;
            security.check(&reopened, false)?;
            let named = check_object(&reopened, true)?;
            if (actual.volume, actual.id) != *expected || (named.volume, named.id) != *expected {
                return Err(StorageError::ExternalChange);
            }
        }
        if self.observe()? != self.identity {
            return Err(StorageError::ExternalChange);
        }
        Ok(())
    }
}
#[cfg(test)]
struct LocationState {
    lock: LocationLock,
    slots: [SlotWatch; 2],
}
#[cfg(test)]
impl LocationState {
    fn revalidate(&self, security: &Security) -> Result<(), StorageError> {
        check_location_mutex(self.lock.0 .0, &location_descriptor(security)?, security)?;
        for slot in &self.slots {
            slot.revalidate(security)?;
        }
        Ok(())
    }
}
#[cfg(test)]
fn probe_slots(paths: &[PathBuf; 2]) -> Result<[Option<(u32, u64)>; 2], StorageError> {
    let security = Security::new()?;
    Ok([
        SlotWatch::new(&paths[0], &security)?.identity,
        SlotWatch::new(&paths[1], &security)?.identity,
    ])
}
fn check_parent_location(
    parent: &File,
    dirs: &[File],
    security: &Security,
) -> Result<(), StorageError> {
    // The direct parent must belong to the current user, not just be writable.
    let mut owner = null_mut();
    let mut sd = null_mut();
    let result = unsafe {
        GetSecurityInfo(
            parent.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sd,
        )
    };
    if result != 0 {
        return Err(error(result));
    }
    let _sd = Allocation(sd);
    if sid_copy(owner)? != security.user {
        return Err(StorageError::UnsafePath);
    }
    check_volume_and_sync(parent, dirs)
}
fn check_volume_and_sync(parent: &File, dirs: &[File]) -> Result<(), StorageError> {
    cloud_check(parent, dirs)?;
    let mut fs = [0u16; 32];
    let mut flags = 0;
    if unsafe {
        GetVolumeInformationByHandleW(
            parent.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            &mut flags,
            fs.as_mut_ptr(),
            fs.len() as u32,
        )
    } == 0
    {
        return Err(last());
    }
    let n = fs
        .iter()
        .position(|v| *v == 0)
        .ok_or(StorageError::UnsafePath)?;
    if String::from_utf16(&fs[..n]).map_err(|_| StorageError::UnsafePath)? != "NTFS" {
        return Err(StorageError::UnsafePath);
    }
    Ok(())
}
impl Disk {
    // Called only with OS-derived fixed paths, or the same fixed child names
    // beneath a test-owned synthetic parent. No public path/trust injection.
    #[cfg(test)]
    fn at_known_location(paths: &[PathBuf; 2], create: bool) -> Result<Self, StorageError> {
        let security = Security::new()?;
        let namespace = known_location_namespace(paths);
        let lock = LocationLock::acquire(&security, &namespace)?;
        Self::at_known_location_locked(paths, create, security, lock)
    }
    fn at_known_location_locked(
        paths: &[PathBuf; 2],
        create: bool,
        security: Security,
        lock: LocationLock,
    ) -> Result<Self, StorageError> {
        let container = paths[1].parent().ok_or(StorageError::UnsafePath)?;
        let profile = container.parent().ok_or(StorageError::UnsafePath)?;
        if paths[0].file_name() != Some(std::ffi::OsStr::new(VAULT_CHILD))
            || paths[1].file_name() != Some(std::ffi::OsStr::new(VAULT_CHILD))
            || container.file_name() != Some(std::ffi::OsStr::new(STORAGE_CONTAINER))
        {
            return Err(StorageError::UnsafePath);
        }
        // Fixed observation order is significant. Occupancy never masks a
        // denied/unsupported/changed earlier metadata acquisition.
        let legacy = LegacyObservation::new(&paths[0], &security)?;
        let reserved = MetadataWatch::new(
            &profile.join(VAULT_CHILD),
            &security,
            ObservationPolicy::StrictAncestors,
        )?;
        let container_watch =
            MetadataWatch::new(container, &security, ObservationPolicy::StrictAncestors)?;
        let nested = if let Some(file) = &container_watch.leaf {
            security.check(file, true)?;
            Some(MetadataWatch::new(
                &paths[1],
                &security,
                ObservationPolicy::StrictAncestors,
            )?)
        } else {
            None
        };
        let mut layout = LayoutState {
            lock,
            legacy,
            reserved,
            container: container_watch,
            nested,
        };
        let legacy_exists = layout.legacy.0.identity.is_some();
        let reserved_exists = layout.reserved.identity.is_some();
        let nested_exists = layout.nested.as_ref().is_some_and(|n| n.identity.is_some());
        let selected = if create {
            if legacy_exists || reserved_exists || nested_exists {
                return Err(StorageError::AlreadyExists);
            }
            if layout.container.identity.is_some() {
                return Err(StorageError::RecoveryRequired);
            }
            1
        } else {
            if reserved_exists || (legacy_exists && nested_exists) {
                return Err(StorageError::ActiveStateUnknown);
            }
            if legacy_exists && layout.container.identity.is_some() {
                return Err(StorageError::RecoveryRequired);
            }
            if legacy_exists {
                0
            } else if nested_exists {
                1
            } else if layout.container.identity.is_some() {
                return Err(StorageError::RecoveryRequired);
            } else {
                return Err(StorageError::Missing);
            }
        };
        if selected == 1 && !create && directory_names(container)? != [VAULT_CHILD] {
            return Err(StorageError::UnsafePath);
        }
        layout.revalidate(&security)?;
        if create {
            if unsafe { CreateDirectoryW(wide(container)?.as_ptr(), &security.attrs()) } == 0 {
                return Err(last());
            }
            layout.container =
                MetadataWatch::new(container, &security, ObservationPolicy::StrictAncestors)?;
            security.check(
                layout
                    .container
                    .leaf
                    .as_ref()
                    .ok_or(StorageError::ExternalChange)?,
                true,
            )?;
            layout.nested = Some(MetadataWatch::new(
                &paths[1],
                &security,
                ObservationPolicy::StrictAncestors,
            )?);
            layout.revalidate(&security)?;
            #[cfg(test)]
            tests::creation_checkpoint(1)?;
            // The root is exclusively created only after the actual protected
            // container handle, identity, descriptor and empty inventory are bound.
            if unsafe { CreateDirectoryW(wide(&paths[1])?.as_ptr(), &security.attrs()) } == 0 {
                return Err(last());
            }
            layout.nested = Some(MetadataWatch::new(
                &paths[1],
                &security,
                ObservationPolicy::StrictAncestors,
            )?);
            layout.revalidate(&security)?;
            #[cfg(test)]
            tests::creation_checkpoint(2)?;
        }
        let mode = if create {
            LockAdmission::CreateNew
        } else {
            LockAdmission::ExistingReadOnly
        };
        let mut disk = Self::at_with_lock(&paths[selected], false, mode, Some(&layout))?;
        disk.bound_security = Some(
            disk.dirs
                .iter()
                .chain([&disk._lock])
                .map(descriptor_bytes)
                .collect::<Result<_, _>>()?,
        );
        disk.layout = Some(layout);
        disk.validate()?;
        #[cfg(test)]
        if create {
            tests::creation_checkpoint(3)?;
        }
        if !create {
            // Lock admission happened before even key eligibility classification.
            let wrapped = match disk.read("key.cakp")? {
                Some(bytes) => {
                    if disk.read("key.cakp.stage")?.is_some() {
                        return Err(StorageError::ExternalChange);
                    }
                    bytes
                }
                None => {
                    disk.validate()?;
                    let names = directory_names(&disk.root)?;
                    disk.validate()?;
                    if names.iter().any(|n| n != "lock" && n != "key.cakp.stage") {
                        return Err(StorageError::KeyUnavailable);
                    }
                    return Err(StorageError::RecoveryRequired);
                }
            };
            let root = RootKey::unprotect(
                &ProtectedRootKey::from_bytes(&wrapped)
                    .map_err(|_| StorageError::KeyUnavailable)?,
            )
            .map_err(|_| StorageError::KeyUnavailable)?;
            let snapshot = disk.recognition_snapshot()?;
            crate::engine::recognize_existing(&disk, &root)?;
            disk.recognized = Some(snapshot);
            disk.check_recognized()?;
        }
        Ok(disk)
    }
    #[cfg(test)]
    fn at_location(
        paths: &[PathBuf; 2],
        create: bool,
        namespace: &[u8],
    ) -> Result<Self, StorageError> {
        use crate::storage_location::{decide, Decision, Operation, SlotState};
        let security = Security::new()?;
        let lock = LocationLock::acquire(&security, namespace)?;
        let mut location = LocationState {
            lock,
            slots: [
                SlotWatch::new(&paths[0], &security)?,
                SlotWatch::new(&paths[1], &security)?,
            ],
        };
        let state = |watch: &SlotWatch| {
            if watch.identity.is_some() {
                SlotState::Directory
            } else {
                SlotState::Absent
            }
        };
        let decision = decide(
            if create {
                Operation::Create
            } else {
                Operation::Open
            },
            state(&location.slots[0]),
            state(&location.slots[1]),
        )?;
        let index = if decision == Decision::OpenLegacy {
            0
        } else {
            1
        };
        location.revalidate(&security)?;
        let mut disk = Self::at(&paths[index], create)?;
        let actual = check_object(disk.dirs.last().ok_or(StorageError::UnsafePath)?, true)?;
        if create {
            location.slots[index].identity = Some((actual.volume, actual.id));
        }
        location.revalidate(&security)?;
        if location.slots[index].identity != Some((actual.volume, actual.id)) {
            return Err(StorageError::ExternalChange);
        }
        disk.location = Some(location);
        disk.validate()?;
        Ok(disk)
    }
    #[cfg(test)]
    fn at(root: &Path, create: bool) -> Result<Self, StorageError> {
        Self::at_with_lock(root, create, LockAdmission::HistoricalBootstrap, None)
    }
    fn at_with_lock(
        root: &Path,
        create: bool,
        mode: LockAdmission,
        layout: Option<&LayoutState>,
    ) -> Result<Self, StorageError> {
        validate_path(root)?;
        let security = Security::new()?;
        let mut drive = PathBuf::new();
        for c in root.components().take(2) {
            drive.push(c.as_os_str());
        }
        if unsafe { GetDriveTypeW(wide(&drive)?.as_ptr()) } != DRIVE_FIXED {
            return Err(StorageError::UnsafePath);
        }
        let mut paths: Vec<_> = root.ancestors().skip(1).collect();
        paths.reverse();
        let mut dirs = vec![];
        for p in paths {
            let f = open_file(
                p,
                FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ,
                OPEN_EXISTING,
                None,
            )?;
            check_object(&f, true)?;
            security.check(&f, false)?;
            dirs.push(f);
        }
        let parent = dirs.last().ok_or(StorageError::UnsafePath)?;
        check_parent_location(parent, &dirs, &security)?;
        if create {
            if let Some(layout) = layout {
                layout.revalidate(&security)?;
            }
            let name = wide(root)?;
            let attrs = security.attrs();
            if unsafe { CreateDirectoryW(name.as_ptr(), &attrs) } == 0 {
                return Err(last());
            }
        }
        let f = open_file(
            root,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )?;
        check_object(&f, true)?;
        security.check(&f, true)?;
        dirs.push(f);
        cloud_check(dirs.last().ok_or(StorageError::UnsafePath)?, &dirs)?;
        let identities = dirs
            .iter()
            .map(|f| stamp(f).map(|s| (s.volume, s.id)))
            .collect::<Result<_, _>>()?;
        let lock_path = root.join("lock");
        if let Some(layout) = layout {
            layout.revalidate(&security)?;
        }
        let (access, creation) = match mode {
            #[cfg(test)]
            LockAdmission::HistoricalBootstrap => {
                (GENERIC_READ | GENERIC_WRITE | READ_CONTROL, OPEN_ALWAYS)
            }
            LockAdmission::CreateNew => (GENERIC_READ | GENERIC_WRITE | READ_CONTROL, CREATE_NEW),
            LockAdmission::ExistingReadOnly => (GENERIC_READ | READ_CONTROL, OPEN_EXISTING),
        };
        let lock = open_file(&lock_path, access, 0, creation, Some(&security)).map_err(|e| {
            if matches!(mode, LockAdmission::ExistingReadOnly) && e == StorageError::Missing {
                StorageError::RecoveryRequired
            } else {
                e
            }
        })?;
        let s = check_object(&lock, false)?;
        security.check(&lock, true)?;
        if s.size != 0 {
            return Err(StorageError::Corrupt);
        }
        if let Some(layout) = layout {
            layout.revalidate(&security)?;
        }
        if !matches!(mode, LockAdmission::ExistingReadOnly) {
            lock.sync_all().map_err(|_| StorageError::Io)?;
        }
        if let Some(layout) = layout {
            layout.revalidate(&security)?;
        }
        Ok(Self {
            root: root.to_owned(),
            dirs,
            identities,
            security,
            _lock: lock,
            #[cfg(test)]
            location: None,
            layout: None,
            bound_security: None,
            recognized: None,
        })
    }
    fn validate(&self) -> Result<(), StorageError> {
        if let Some(layout) = &self.layout {
            layout.revalidate(&self.security)?;
        }
        if let Some(expected) = &self.bound_security {
            for (file, bytes) in self.dirs.iter().chain([&self._lock]).zip(expected) {
                if descriptor_bytes(file)? != *bytes {
                    return Err(StorageError::ExternalChange);
                }
            }
            if check_object(&self._lock, false)?.size != 0 {
                return Err(StorageError::ExternalChange);
            }
        }
        #[cfg(test)]
        if let Some(location) = &self.location {
            location.revalidate(&self.security)?;
        }
        for (index, (f, id)) in self.dirs.iter().zip(&self.identities).enumerate() {
            self.security.check(f, index + 1 == self.dirs.len())?;
            let s = check_object(f, true)?;
            if (s.volume, s.id) != *id {
                return Err(StorageError::ExternalChange);
            }
        }
        let root = self.dirs.last().ok_or(StorageError::UnsafePath)?;
        self.security.check(root, true)?;
        cloud_check(root, &self.dirs)
    }
    fn recognition_snapshot(&self) -> Result<Vec<RecognizedLeaf>, StorageError> {
        let mut result = Vec::new();
        for name in self.list()? {
            if name == "lock" {
                continue;
            }
            let mut file = self.leaf(&name, GENERIC_READ)?;
            let bytes = read_bytes(&mut file)?;
            result.push(RecognizedLeaf {
                name,
                stamp: stamp(&file)?,
                descriptor: descriptor_bytes(&file)?,
                hash: digest(&bytes),
            });
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }
    fn check_recognized(&self) -> Result<(), StorageError> {
        if let Some(expected) = &self.recognized {
            let actual = self.recognition_snapshot()?;
            if actual.len() != expected.len()
                || actual.iter().zip(expected).any(|(a, b)| {
                    a.name != b.name
                        || a.stamp != b.stamp
                        || a.descriptor != b.descriptor
                        || a.hash != b.hash
                })
            {
                return Err(StorageError::ExternalChange);
            }
        }
        Ok(())
    }
    fn admit_effect(&mut self) -> Result<(), StorageError> {
        self.validate()?;
        self.check_recognized()?;
        self.recognized = None;
        Ok(())
    }
    fn path(&self, name: &str) -> Result<PathBuf, StorageError> {
        // Private trait still rechecks the path grammar at the native boundary.
        if name.is_empty()
            || name.len() > 110
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
            || name.starts_with('.')
            || name.contains("..")
        {
            return Err(StorageError::UnsafePath);
        }
        Ok(self.root.join(name))
    }
    fn leaf(&self, name: &str, access: u32) -> Result<File, StorageError> {
        self.validate()?;
        let f = open_file(
            &self.path(name)?,
            access | READ_CONTROL,
            0,
            OPEN_EXISTING,
            None,
        )?;
        check_object(&f, false)?;
        self.security.check(&f, true)?;
        Ok(f)
    }
}
impl Files for Disk {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let mut f = match self.leaf(name, GENERIC_READ) {
            Ok(f) => f,
            Err(StorageError::Missing) => {
                if self
                    .recognized
                    .as_ref()
                    .is_some_and(|rows| rows.iter().any(|r| r.name == name))
                {
                    return Err(StorageError::ExternalChange);
                }
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        let b = read_bytes(&mut f)?;
        if let Some(rows) = &self.recognized {
            let expected = rows
                .iter()
                .find(|r| r.name == name)
                .ok_or(StorageError::ExternalChange)?;
            if stamp(&f)? != expected.stamp
                || descriptor_bytes(&f)? != expected.descriptor
                || digest(&b) != expected.hash
            {
                return Err(StorageError::ExternalChange);
            }
        }
        self.validate()?;
        Ok(Some(b))
    }
    fn stage(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        if bytes.len() > MAX_FILE {
            return Err(StorageError::InputLimit);
        }
        self.admit_effect()?;
        let name = format!("{name}.stage");
        if let Some(old) = self.read(&name)? {
            if old == bytes {
                return Ok(());
            }
            return Err(StorageError::ExternalChange);
        }
        self.validate()?;
        let mut f = open_file(
            &self.path(&name)?,
            GENERIC_READ | GENERIC_WRITE | DELETE | READ_CONTROL,
            0,
            CREATE_NEW,
            Some(&self.security),
        )?;
        check_object(&f, false)?;
        self.security.check(&f, true)?;
        self.validate()?;
        f.write_all(bytes).map_err(|_| StorageError::Io)?;
        self.validate()?;
        f.sync_all().map_err(|_| StorageError::Io)?;
        self.validate()?;
        if read_bytes(&mut f)? != bytes {
            return Err(StorageError::ExternalChange);
        }
        self.security.check(&f, true)?;
        self.validate()
    }
    fn publish(
        &mut self,
        name: &str,
        expected: Option<&[u8]>,
        staged: &[u8],
    ) -> Result<(), StorageError> {
        self.admit_effect()?;
        self.validate()?;
        let current = self.read(name)?;
        if current.as_deref() != expected {
            return Err(StorageError::ExternalChange);
        }
        let mut source = self.leaf(
            &format!("{name}.stage"),
            GENERIC_READ | GENERIC_WRITE | DELETE,
        )?;
        if read_bytes(&mut source)? != staged {
            return Err(StorageError::ExternalChange);
        }
        let before = stamp(&source)?;
        let target: Vec<u16> = self
            .path(name)?
            .as_os_str()
            .encode_wide()
            .chain([0])
            .collect();
        let offset = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
        let size = offset + target.len() * 2;
        let mut storage = vec![0u64; size.div_ceil(8)];
        let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        // SAFETY: aligned zeroed variable-length SDK structure. Full path lies
        // under retained no-delete ancestors. Source handle pins the staged file.
        unsafe {
            (*info).Anonymous.ReplaceIfExists = expected.is_some();
            (*info).RootDirectory = null_mut();
            (*info).FileNameLength = ((target.len() - 1) * 2) as u32;
            std::ptr::copy_nonoverlapping(
                target.as_ptr(),
                (*info).FileName.as_mut_ptr(),
                target.len(),
            );
        }
        // Re-observe immediately before rename. The kernel lock serializes our
        // instances; arbitrary same-user ACL changes are outside exclusion proof.
        if self.read(name)?.as_deref() != expected {
            return Err(StorageError::ExternalChange);
        }
        if unsafe {
            SetFileInformationByHandle(
                source.as_raw_handle(),
                FileRenameInfo,
                info.cast(),
                size as u32,
            )
        } == 0
        {
            return Err(last());
        }
        self.validate()?;
        source.sync_all().map_err(|_| StorageError::Io)?;
        self.validate()?;
        self.security.check(&source, true)?;
        let after = stamp(&source)?;
        if after.id != before.id
            || after.volume != before.volume
            || read_bytes(&mut source)? != staged
        {
            return Err(StorageError::ExternalChange);
        }
        drop(source);
        if self.read(name)?.as_deref() != Some(staged) {
            return Err(StorageError::ExternalChange);
        }
        self.validate()?;
        #[cfg(test)]
        if name == "state.bin" {
            tests::creation_checkpoint(5)?;
        }
        Ok(())
    }
    fn erase(&mut self, name: &str, expected: &[u8]) -> Result<(), StorageError> {
        self.admit_effect()?;
        let mut f = self.leaf(name, GENERIC_READ | GENERIC_WRITE | DELETE)?;
        if read_bytes(&mut f)? != expected {
            return Err(StorageError::ExternalChange);
        }
        self.validate()?;
        let info = FILE_DISPOSITION_INFO { DeleteFile: true };
        // SAFETY: owned validated file handle and correctly sized SDK value.
        if unsafe {
            SetFileInformationByHandle(
                f.as_raw_handle(),
                FileDispositionInfo,
                (&info as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of_val(&info) as u32,
            )
        } == 0
        {
            return Err(last());
        }
        self.validate()?;
        f.sync_all().map_err(|_| StorageError::Io)?;
        self.validate()?;
        drop(f);
        self.validate()
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        self.validate()?;
        let mut result = vec![];
        for item in std::fs::read_dir(&self.root).map_err(|_| StorageError::Io)? {
            let item = item.map_err(|_| StorageError::Io)?;
            if result.len() >= MAX_BLOBS * 2 + 3 {
                return Err(StorageError::InputLimit);
            }
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| StorageError::UnsafePath)?;
            if name != "lock" {
                let _ = self.leaf(&name, FILE_READ_ATTRIBUTES)?;
            }
            result.push(name);
        }
        self.validate()?;
        Ok(result)
    }
}

fn bootstrap(disk: &mut impl Files, create: bool) -> Result<RootKey, StorageError> {
    if create {
        if disk.list()?.iter().any(|n| n != "lock") {
            return Err(StorageError::AlreadyExists);
        }
        let root = RootKey::generate().map_err(|_| StorageError::KeyUnavailable)?;
        let wrapped = root.protect().map_err(|_| StorageError::KeyUnavailable)?;
        disk.stage("key.cakp", wrapped.as_bytes())?;
        disk.publish("key.cakp", None, wrapped.as_bytes())?;
        #[cfg(test)]
        tests::creation_checkpoint(4)?;
        return Ok(root);
    }
    let wrapped = match disk.read("key.cakp")? {
        Some(b) => {
            if disk.read("key.cakp.stage")?.is_some() {
                return Err(StorageError::ExternalChange);
            }
            b
        }
        None => {
            // Only the first bootstrap's staged native-protected key is eligible.
            // Existing account/registry artifacts forbid adopting a different key.
            if disk
                .list()?
                .iter()
                .any(|n| n != "lock" && n != "key.cakp.stage")
            {
                return Err(StorageError::KeyUnavailable);
            }
            let stage = disk
                .read("key.cakp.stage")?
                .ok_or(StorageError::KeyUnavailable)?;
            let parsed =
                ProtectedRootKey::from_bytes(&stage).map_err(|_| StorageError::KeyUnavailable)?;
            RootKey::unprotect(&parsed).map_err(|_| StorageError::KeyUnavailable)?;
            disk.publish("key.cakp", None, &stage)?;
            stage
        }
    };
    RootKey::unprotect(
        &ProtectedRootKey::from_bytes(&wrapped).map_err(|_| StorageError::KeyUnavailable)?,
    )
    .map_err(|_| StorageError::KeyUnavailable)
}
pub(crate) fn open_vault(create: bool) -> Result<Vault, StorageError> {
    open_vault_dispatch(create, b"application", fixed_locations)
}
fn open_vault_dispatch(
    create: bool,
    namespace: &[u8],
    resolve: impl FnOnce() -> Result<[PathBuf; 2], StorageError>,
) -> Result<Vault, StorageError> {
    let security = Security::new()?;
    let lock = LocationLock::acquire(&security, namespace)?;
    let paths = resolve()?;
    finish_vault(
        Disk::at_known_location_locked(&paths, create, security, lock)?,
        create,
    )
}
fn finish_vault(mut disk: Disk, create: bool) -> Result<Vault, StorageError> {
    let root = bootstrap(&mut disk, create)?;
    disk.check_recognized()?;
    let storage = if create {
        Storage::create(disk, &root)?
    } else {
        Storage::open(disk, &root)?
    };
    Ok(Vault { root, storage })
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;

// Private native primitives; no qualified production caller or public executor.
#[allow(dead_code)]
mod lifecycle;

// Only a compiled test can vary the namespace, through its uniquely owned paths.
// Production always uses the fixed application namespace above.
#[cfg(test)]
fn known_location_namespace(paths: &[PathBuf; 2]) -> Vec<u8> {
    paths[1]
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}
