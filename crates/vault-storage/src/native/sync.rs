//! Read-only sync-location evidence. API failure is not evidence of absence.
use super::*;

// Stack-owned and thread-affine: never run COM teardown in a TLS destructor.
// Windows holds its loader lock while Rust thread-local destructors execute.
struct Apartment(std::marker::PhantomData<std::rc::Rc<()>>);
impl Apartment {
    fn enter() -> Result<Self, StorageError> {
        // SAFETY: each successful call is balanced on this same thread. An
        // incompatible existing apartment is refused, never reconfigured.
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
        // SAFETY: same-thread stack guard, after all scoped WinRT interfaces drop,
        // before thread exit acquires the loader lock. Never stored in thread_local!.
        unsafe { windows::Win32::System::WinRT::RoUninitialize() };
    }
}

pub(super) fn registered_sync_check(ancestors: &[File]) -> Result<(), StorageError> {
    use windows::core::{Interface, HSTRING};
    use windows::Storage::{IStorageItem, Provider::IStorageProviderSyncRootManagerStatics};
    // Declared first, dropped last on every return and unwind. Nested successful
    // initializations (including S_FALSE) each retain their own balancing guard.
    let _apartment = Apartment::enter()?;
    // This must succeed positively. A failed enumeration is NEVER no registered roots.
    // Microsoft documents both legacy and modern registrations in this inventory.
    // Request the registered factory directly: the generated static helper caches
    // an agile factory across apartment teardown. This scoped factory and every
    // derived interface are released before the stack-owned apartment. There is no
    // generated helper's DLL-search fallback for a missing registered class.
    let factory: IStorageProviderSyncRootManagerStatics = unsafe {
        windows::Win32::System::WinRT::RoGetActivationFactory(&HSTRING::from(
            "Windows.Storage.Provider.StorageProviderSyncRootManager",
        ))
    }
    .map_err(|_| StorageError::UnsafePath)?;
    // SAFETY: use the generated SDK vtable and output type. The live factory
    // owns the call; null-initialized output is converted only after success.
    let roots: windows_collections::IVectorView<
        windows::Storage::Provider::StorageProviderSyncRootInfo,
    > = unsafe {
        let mut output = null_mut();
        (factory.vtable().GetCurrentSyncRoots)(factory.as_raw(), &mut output)
            .and_then(|| windows::core::Type::from_abi(output))
    }
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
        // Inspect every ancestor without following a link, before the next
        // component is opened. No secret reads, remote drive open or hydration.
        let mut paths: Vec<_> = path.ancestors().collect();
        paths.reverse();
        let mut held = Vec::new();
        for part in paths {
            let file = open_file(
                part,
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                OPEN_EXISTING,
                None,
            )?;
            check_object(&file, true)?;
            held.push(file);
        }
        let observed = stamp(held.last().ok_or(StorageError::UnsafePath)?)?;
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

#[cfg(test)]
#[path = "creation_checks.rs"]
mod creation_checks;

#[cfg(test)]
#[test]
fn native_repeated_inventory_retains_the_owned_thread_apartment() {
    // Explicit stack ownership retains the apartment across this nested batch.
    // Every query still gets an uncached factory and independently balanced guard.
    let _apartment = Apartment::enter().expect("owned batch apartment");
    for _ in 0..100 {
        registered_sync_check(&[]).expect("independent registered-root inventory");
    }
}

#[cfg(test)]
#[test]
fn native_inventory_thread_exit_balances_com_before_tls_teardown() {
    // Exercise concurrent fresh thread lifetimes, not merely a reused test thread.
    // No COM object or apartment crosses a thread boundary or survives its stack.
    let children: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..8 {
                    registered_sync_check(&[]).expect("fresh thread inventory");
                }
            })
        })
        .collect();
    for child in children {
        child.join().expect("inventory thread completed");
    }
}
