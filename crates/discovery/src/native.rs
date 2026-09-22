//! The only application-authored unsafe boundary. Read-only Win32 operations.
//! Handles and buffers outlive synchronous calls; every successful allocation is
//! owned exactly once. See docs/ca-02-native-boundary.md for API/race limitations.
use crate::{ReadError, MAX_EXECUTABLE_BYTES, MAX_PATH_UNITS};
use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::{Component, Path, PathBuf, Prefix};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::Security::WinTrust::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

pub struct Apartment(std::marker::PhantomData<std::rc::Rc<()>>);
impl Apartment {
    pub fn enter() -> Result<Self, ReadError> {
        // SAFETY: Called on this thread and balanced on this same thread. The Rc
        // marker prevents sending the guard; a changed apartment is a refusal.
        let result = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        if result < 0 { return Err(ReadError::EnumerationUnavailable); }
        Ok(Self(std::marker::PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: This non-Send guard owns one successful RoInitialize call.
        unsafe { RoUninitialize() };
    }
}

pub fn error(code: u32) -> ReadError {
    match code {
        ERROR_ACCESS_DENIED => ReadError::AccessDenied,
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => ReadError::Missing,
        ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION => ReadError::SharingViolation,
        _ => ReadError::Io,
    }
}
fn last_error() -> ReadError {
    // SAFETY: Thread-local error query, no pointer arguments.
    error(unsafe { GetLastError() })
}

fn wide(path: &Path) -> Result<Vec<u16>, ReadError> {
    let mut units: Vec<u16> = path.as_os_str().encode_wide().collect();
    if units.is_empty() || units.len() >= MAX_PATH_UNITS || units.contains(&0) {
        return Err(ReadError::UnsafePath);
    }
    units.push(0);
    Ok(units)
}

pub fn validate_absolute(path: &Path) -> Result<(), ReadError> {
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)))
        || !matches!(components.next(), Some(Component::RootDir)) {
        return Err(ReadError::UnsafePath);
    }
    for component in components {
        let Component::Normal(part) = component else { return Err(ReadError::UnsafePath); };
        let part = part.to_string_lossy();
        if part.contains([':', '\0']) || part.ends_with(['.', ' ']) || part.chars().any(char::is_control) {
            return Err(ReadError::UnsafePath);
        }
        let name = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL"].contains(&name.as_str())
            || (name.len() == 4 && (name.starts_with("COM") || name.starts_with("LPT")) && name.as_bytes()[3].is_ascii_digit()) {
            return Err(ReadError::UnsafePath);
        }
    }
    // Components normalizes explicit '.'; reject it in the supplied spelling too.
    let text = path.to_string_lossy();
    if text.split(['/', '\\']).any(|part| part == "." || part == "..") { return Err(ReadError::UnsafePath); }
    wide(path)?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub volume: u32,
    pub file_id: u64,
    pub size: u64,
    pub modified: u64,
    pub attributes: u32,
    pub links: u32,
}

fn stamp(file: &File) -> Result<Stamp, ReadError> {
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: Valid borrowed handle; initialized, correctly sized output storage.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 { return Err(last_error()); }
    Ok(Stamp { volume: info.dwVolumeSerialNumber,
        file_id: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        size: ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64,
        modified: ((info.ftLastWriteTime.dwHighDateTime as u64) << 32) | info.ftLastWriteTime.dwLowDateTime as u64,
        attributes: info.dwFileAttributes, links: info.nNumberOfLinks })
}

fn open_one(path: &Path, directory: bool) -> Result<File, ReadError> {
    let name = wide(path)?;
    let desired = if directory { FILE_READ_ATTRIBUTES | READ_CONTROL } else { GENERIC_READ | READ_CONTROL };
    let share = if directory { FILE_SHARE_READ | FILE_SHARE_WRITE } else { FILE_SHARE_READ };
    // SAFETY: NUL-terminated UTF-16 survives the synchronous call. OPEN_EXISTING
    // and read-only access never create or alter the target. Reparse leaves are
    // opened as links and rejected below; directory handles pin all ancestors.
    let handle = unsafe { CreateFileW(name.as_ptr(), desired, share, null(), OPEN_EXISTING,
        FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS, null_mut()) };
    if handle == INVALID_HANDLE_VALUE { return Err(last_error()); }
    // SAFETY: CreateFileW returned one new owned handle, transferred to File.
    let file = unsafe { File::from_raw_handle(handle) };
    // SAFETY: Handle remains owned by file.
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK { return Err(ReadError::UnsafePath); }
    let info = stamp(&file)?;
    if info.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (info.attributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
        || (!directory && info.links != 1) { return Err(ReadError::UnsafePath); }
    Ok(file)
}

/// Pins the path's directories against rename and the leaf against write/delete.
/// Read observations are not the later complete protected-write/vault adapter.
pub struct ReadHandle {
    pub file: File,
    ancestors: Vec<File>,
    path: PathBuf,
    initial: Stamp,
    directory: bool,
}
impl std::fmt::Debug for ReadHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("ReadHandle([REDACTED])") }
}
impl ReadHandle {
    pub fn open(path: &Path, directory: bool) -> Result<Self, ReadError> {
        validate_absolute(path)?;
        let mut root = PathBuf::new();
        for component in path.components().take(2) { root.push(component.as_os_str()); }
        let root_wide = wide(&root)?;
        // SAFETY: NUL-terminated absolute drive root, no network target accepted.
        if unsafe { GetDriveTypeW(root_wide.as_ptr()) } != DRIVE_FIXED { return Err(ReadError::UnsafePath); }
        let mut ancestors = Vec::new();
        let mut chain: Vec<&Path> = path.ancestors().skip(1).collect();
        chain.reverse();
        for parent in chain { ancestors.push(open_one(parent, true)?); }
        let file = open_one(path, directory)?;
        let initial = stamp(&file)?;
        if !directory && initial.size > MAX_EXECUTABLE_BYTES { return Err(ReadError::InputLimit); }
        Ok(Self { file, ancestors, path: path.to_owned(), initial, directory })
    }
    pub fn identity(&self) -> Stamp { self.initial }
    pub fn revalidate(&self) -> Result<(), ReadError> {
        if stamp(&self.file)? != self.initial { return Err(ReadError::Changed); }
        let current = open_one(&self.path, self.directory)?;
        if stamp(&current)? != self.initial { return Err(ReadError::Changed); }
        Ok(())
    }
    pub fn bytes(&mut self, limit: usize) -> Result<Vec<u8>, ReadError> {
        if self.directory || self.initial.size > limit as u64 { return Err(ReadError::InputLimit); }
        self.file.seek(SeekFrom::Start(0)).map_err(|_| ReadError::Io)?;
        let mut bytes = Vec::new();
        (&mut self.file).take(limit as u64 + 1).read_to_end(&mut bytes).map_err(|_| ReadError::Io)?;
        if bytes.len() > limit || bytes.len() as u64 != self.initial.size { return Err(ReadError::Changed); }
        self.revalidate()?;
        Ok(bytes)
    }
    pub fn current_user_can_replace_or_modify(&self) -> Result<bool, ReadError> {
        if writable(&self.file, self.directory)? { return Ok(true); }
        for ancestor in &self.ancestors {
            if writable(ancestor, true)? { return Ok(true); }
        }
        Ok(false)
    }
    pub fn cached_signature(&self) -> bool {
        let Ok(name) = wide(&self.path) else { return false; };
        // SAFETY: All-zero defaults have valid null optional pointers. All live
        // pointers below refer to owned stack/file data for both synchronous calls.
        let mut file_info: WINTRUST_FILE_INFO = unsafe { std::mem::zeroed() };
        file_info.cbStruct = std::mem::size_of::<WINTRUST_FILE_INFO>() as u32;
        file_info.pcwszFilePath = name.as_ptr();
        file_info.hFile = self.file.as_raw_handle();
        let mut data: WINTRUST_DATA = unsafe { std::mem::zeroed() };
        data.cbStruct = std::mem::size_of::<WINTRUST_DATA>() as u32;
        data.dwUIChoice = WTD_UI_NONE;
        data.fdwRevocationChecks = WTD_REVOKE_WHOLECHAIN;
        data.dwUnionChoice = WTD_CHOICE_FILE;
        data.Anonymous.pFile = &mut file_info;
        data.dwStateAction = WTD_STATEACTION_VERIFY;
        data.dwProvFlags = WTD_CACHE_ONLY_URL_RETRIEVAL | WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT | WTD_DISABLE_MD2_MD4;
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        // SAFETY: Handles and all nested pointers live through verification and
        // cleanup. Cached-only retrieval forbids network certificate retrieval.
        let status = unsafe { WinVerifyTrust(null_mut(), &mut action, (&mut data as *mut WINTRUST_DATA).cast()) };
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        // SAFETY: Required matching close, including when verification failed.
        unsafe { WinVerifyTrust(null_mut(), &mut action, (&mut data as *mut WINTRUST_DATA).cast()) };
        status == 0
    }
}

struct Token(HANDLE);
impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: Non-null successful token handle owned only by this guard.
        unsafe { CloseHandle(self.0) };
    }
}
struct Descriptor(*mut c_void);
impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: GetSecurityInfo returns a LocalAlloc allocation, owned here.
        unsafe { LocalFree(self.0) };
    }
}

fn writable(file: &File, directory: bool) -> Result<bool, ReadError> {
    let mut descriptor = null_mut();
    // SAFETY: Output pointer is valid and all unused outputs are null. File is
    // opened with READ_CONTROL. No security descriptor is modified.
    let result = unsafe { GetSecurityInfo(file.as_raw_handle(), SE_FILE_OBJECT,
        DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
        null_mut(), null_mut(), null_mut(), null_mut(), &mut descriptor) };
    if result != ERROR_SUCCESS { return Err(error(result)); }
    if descriptor.is_null() { return Err(ReadError::InvalidMetadata); }
    let descriptor = Descriptor(descriptor);
    let mut process_token = null_mut();
    // SAFETY: Read/duplicate access to this process's token only. No impersonation
    // is installed on a thread, and no privilege or ACL is adjusted.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY | TOKEN_DUPLICATE, &mut process_token) } == 0 { return Err(last_error()); }
    let process_token = Token(process_token);
    let mut token = null_mut();
    // SAFETY: Produces one owned impersonation token used only by AccessCheck.
    if unsafe { DuplicateToken(process_token.0, SecurityImpersonation, &mut token) } == 0 { return Err(last_error()); }
    let token = Token(token);
    let mapping = GENERIC_MAPPING { GenericRead: FILE_GENERIC_READ, GenericWrite: FILE_GENERIC_WRITE,
        GenericExecute: FILE_GENERIC_EXECUTE, GenericAll: FILE_ALL_ACCESS };
    let mut mapping = mapping;
    // Alignment satisfies PRIVILEGE_SET and capacity is bounded; insufficient
    // capacity fails closed rather than assuming no writable grant.
    let mut privileges = [0u64; 128];
    let mut length = std::mem::size_of_val(&privileges) as u32;
    let mut granted = 0;
    let mut access = 0;
    // SAFETY: Descriptor remains allocated, token valid, all output pointers have
    // sufficient alignment and the exact byte capacity supplied to AccessCheck.
    if unsafe { AccessCheck(descriptor.0, token.0, MAXIMUM_ALLOWED, &mut mapping,
        privileges.as_mut_ptr().cast(), &mut length, &mut granted, &mut access) } == 0 { return Err(last_error()); }
    if access == 0 { return Err(ReadError::AccessDenied); }
    let unsafe_rights = if directory { DELETE | FILE_DELETE_CHILD | WRITE_DAC | WRITE_OWNER }
        else { DELETE | FILE_WRITE_DATA | FILE_APPEND_DATA | FILE_WRITE_EA | FILE_WRITE_ATTRIBUTES | WRITE_DAC | WRITE_OWNER };
    Ok(granted & unsafe_rights != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("ca02-synthetic-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
            std::fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    #[test]
    fn native_read_preserves_bytes_and_reports_user_writability() {
        let fixture = Fixture::new();
        let path = fixture.0.join("合成.exe");
        std::fs::write(&path, b"SYNTHETIC_DATA").unwrap();
        let mut handle = ReadHandle::open(&path, false).unwrap();
        assert_eq!(handle.bytes(128).unwrap(), b"SYNTHETIC_DATA");
        assert!(handle.current_user_can_replace_or_modify().unwrap());
        assert!(!handle.cached_signature());
        assert!(handle.revalidate().is_ok());
    }

    #[test]
    fn native_hardlinks_and_sharing_writers_are_refused() {
        let fixture = Fixture::new();
        let path = fixture.0.join("one.exe");
        std::fs::write(&path, b"SYNTHETIC").unwrap();
        let writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        assert!(matches!(ReadHandle::open(&path, false), Err(ReadError::SharingViolation)));
        drop(writer);
        std::fs::hard_link(&path, fixture.0.join("two.exe")).unwrap();
        assert!(matches!(ReadHandle::open(&path, false), Err(ReadError::UnsafePath)));
    }

    #[test]
    fn native_read_handle_pins_leaf_against_replacement() {
        let fixture = Fixture::new();
        let path = fixture.0.join("old.exe");
        let new = fixture.0.join("new.exe");
        std::fs::write(&path, b"SYNTHETIC_A").unwrap();
        let handle = ReadHandle::open(&path, false).unwrap();
        assert!(std::fs::rename(&path, &new).is_err());
        assert!(handle.revalidate().is_ok());
        drop(handle);
        std::fs::rename(&path, &new).unwrap();
        assert!(matches!(ReadHandle::open(&path, false), Err(ReadError::Missing)));
    }

    #[test]
    fn native_directories_and_data_bounds_are_checked() {
        let fixture = Fixture::new();
        assert!(matches!(ReadHandle::open(&fixture.0, false), Err(ReadError::UnsafePath)));
        let path = fixture.0.join("config.toml");
        std::fs::write(&path, b"SYNTHETIC_DATA").unwrap();
        assert_eq!(ReadHandle::open(&path, false).unwrap().bytes(2), Err(ReadError::InputLimit));
        for path in ["relative.exe", "\\\\server\\share\\x.exe", "\\\\?\\C:\\x.exe", "C:\\a\\..\\x.exe", "C:\\a\\.\\x.exe", "C:\\a.exe:secret"] {
            assert_eq!(validate_absolute(Path::new(path)), Err(ReadError::UnsafePath));
        }
    }
}
