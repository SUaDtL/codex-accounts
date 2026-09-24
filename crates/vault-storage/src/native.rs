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
    UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
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
fn local_root() -> Result<PathBuf, StorageError> {
    let mut p = null_mut();
    // SAFETY: current user's read-only known-folder query. No create/redirect flag.
    let status = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, null_mut(), &mut p) };
    let p = TaskString(p);
    if status < 0 || p.0.is_null() {
        return Err(StorageError::UnsafePath);
    }
    // SAFETY: OS-owned NUL terminated output, bounded scan, freed by TaskString.
    let mut len = 0;
    while len < 240 && unsafe { *p.0.add(len) } != 0 {
        len += 1;
    }
    if len == 240 {
        return Err(StorageError::UnsafePath);
    }
    let path = PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(p.0, len)
    }));
    Ok(path.join("CodexAccounts-Vault-v1"))
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
}
impl Disk {
    fn at(root: &Path, create: bool) -> Result<Self, StorageError> {
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
        cloud_check(parent, &dirs)?;
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
        if create {
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
        let lock = open_file(
            &lock_path,
            GENERIC_READ | GENERIC_WRITE | READ_CONTROL,
            0,
            OPEN_ALWAYS,
            Some(&security),
        )?;
        let s = check_object(&lock, false)?;
        security.check(&lock, true)?;
        if s.size != 0 {
            return Err(StorageError::Corrupt);
        }
        lock.sync_all().map_err(|_| StorageError::Io)?;
        Ok(Self {
            root: root.to_owned(),
            dirs,
            identities,
            security,
            _lock: lock,
        })
    }
    fn validate(&self) -> Result<(), StorageError> {
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
            Err(StorageError::Missing) => return Ok(None),
            Err(e) => return Err(e),
        };
        let b = read_bytes(&mut f)?;
        self.validate()?;
        Ok(Some(b))
    }
    fn stage(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError> {
        if bytes.len() > MAX_FILE {
            return Err(StorageError::InputLimit);
        }
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
        f.write_all(bytes).map_err(|_| StorageError::Io)?;
        f.sync_all().map_err(|_| StorageError::Io)?;
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
        source.sync_all().map_err(|_| StorageError::Io)?;
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
        self.validate()
    }
    fn erase(&mut self, name: &str, expected: &[u8]) -> Result<(), StorageError> {
        let mut f = self.leaf(name, GENERIC_READ | GENERIC_WRITE | DELETE)?;
        if read_bytes(&mut f)? != expected {
            return Err(StorageError::ExternalChange);
        }
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
        f.sync_all().map_err(|_| StorageError::Io)?;
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
    let mut disk = Disk::at(&local_root()?, create)?;
    let root = bootstrap(&mut disk, create)?;
    // A committed bootstrap key with no other payload can safely create only the
    // first empty registry. Never regenerate the key or ignore unexpected files.
    let initial = disk.list()?.iter().all(|n| n == "lock" || n == "key.cakp");
    let storage = if initial {
        Storage::create(disk, &root)?
    } else {
        Storage::open(disk, &root)?
    };
    Ok(Vault { root, storage })
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
