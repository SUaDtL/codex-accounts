//! Read-only registered cloud-root evidence; no registration or service mutation.
use super::*;

struct Apartment(std::marker::PhantomData<std::rc::Rc<()>>);
impl Apartment {
    fn enter() -> Result<Self, StorageError> {
        // SAFETY: a same-thread, non-Send guard balances a successful WinRT init.
        // Changed apartment/unavailable WinRT is a refusal, not an empty inventory.
        unsafe {
            windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            )
        }
        .map_err(|_| StorageError::UnsafePath)?;
        Ok(Self(std::marker::PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: successful same-thread initialization is owned by this guard.
        unsafe { windows::Win32::System::WinRT::RoUninitialize() };
    }
}
pub(super) fn registered_sync_check(ancestors: &[File]) -> Result<(), StorageError> {
    use windows::core::Interface;
    use windows::Storage::{IStorageItem, Provider::StorageProviderSyncRootManager};
    let _apartment = Apartment::enter()?;
    // This must succeed positively. A failed enumeration is NEVER no registered roots.
    // Microsoft documents both legacy and modern registrations in this inventory.
    let roots = StorageProviderSyncRootManager::GetCurrentSyncRoots()
        .map_err(|_| StorageError::UnsafePath)?;
    let count = roots.Size().map_err(|_| StorageError::UnsafePath)?;
    if count > 128 {
        return Err(StorageError::InputLimit);
    }
    let ids = ancestors.iter().map(stamp).collect::<Result<Vec<_>, _>>()?;
    for i in 0..count {
        let root = roots.GetAt(i).map_err(|_| StorageError::UnsafePath)?;
        let folder = root.Path().map_err(|_| StorageError::UnsafePath)?;
        let item: IStorageItem = folder.cast().map_err(|_| StorageError::UnsafePath)?;
        let path = item.Path().map_err(|_| StorageError::UnsafePath)?;
        if path.is_empty() || path.len() > 240 {
            return Err(StorageError::UnsafePath);
        }
        let path = PathBuf::from(OsString::from_wide(&path));
        let mut parts = path.components();
        if !matches!(parts.next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)))
            || !matches!(parts.next(), Some(Component::RootDir))
        {
            return Err(StorageError::UnsafePath);
        }
        // Metadata only, no recall or provider hydration. Inaccessible, remote,
        // ambiguous and reparse-root observations refuse, not silently disappear.
        let file = open_file(
            &path,
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            None,
        )?;
        let observed = check_object(&file, true)?;
        if ids
            .iter()
            .any(|s| (s.volume, s.id) == (observed.volume, observed.id))
        {
            return Err(StorageError::UnsafePath);
        }
    }
    if roots.Size().map_err(|_| StorageError::UnsafePath)? != count {
        return Err(StorageError::ExternalChange);
    }
    Ok(())
}
pub(super) fn cloud_check(f: &File, ancestors: &[File]) -> Result<(), StorageError> {
    let mut info = [0u64; 256];
    let mut size = 0;
    // SAFETY: read-only query on held metadata handle; all returned data stays local.
    let result = unsafe {
        CfGetSyncRootInfoByHandle(
            f.as_raw_handle(),
            CF_SYNC_ROOT_INFO_BASIC,
            info.as_mut_ptr().cast(),
            std::mem::size_of_val(&info) as u32,
            &mut size,
        )
    };
    let hresult = |code: u32| (0x80070000u32 | code) as i32;
    if result == hresult(ERROR_NOT_A_CLOUD_FILE)
        || result == hresult(ERROR_CLOUD_FILE_NOT_UNDER_SYNC_ROOT)
        || result == hresult(ERROR_INVALID_FUNCTION)
    {
        // ERROR_INVALID_FUNCTION is not absence proof. The distinct successful
        // inventory below must establish no registered containing root, including
        // legacy roots that the CloudFilter API alone does not cover.
        registered_sync_check(ancestors)
    } else {
        Err(StorageError::UnsafePath)
    }
}
