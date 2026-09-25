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
        .map_err(|e| api_error("initialize", e))?;
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
fn api_error(_stage: &'static str, _error: windows::core::Error) -> StorageError {
    #[cfg(test)]
    diagnostics::record(_stage, _error.code().0);
    StorageError::UnsafePath
}

// COM may reject activation while its object server is stopping. Retry only
// that explicit transient result, without dropping this call's apartment. This
// is not a path-check retry or an assumption that a failed inventory is empty.
const ACTIVATION_ATTEMPTS: usize = 4;
const ACTIVATION_PAUSE: std::time::Duration = std::time::Duration::from_millis(50);
fn activate_registered<T>(
    mut activate: impl FnMut() -> windows::core::Result<T>,
    mut pause: impl FnMut(std::time::Duration),
) -> windows::core::Result<T> {
    for _ in 1..ACTIVATION_ATTEMPTS {
        match activate() {
            Err(error) if error.code().0 == CO_E_SERVER_STOPPING => pause(ACTIVATION_PAUSE),
            result => return result,
        }
    }
    // Exhaustion returns the actual error. No additional wait or fallback.
    activate()
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
    let factory: IStorageProviderSyncRootManagerStatics = activate_registered(
        || {
            // SAFETY: documented fixed registered class, generated SDK interface;
            // every attempt stays inside the same initialized stack apartment.
            unsafe {
                windows::Win32::System::WinRT::RoGetActivationFactory(&HSTRING::from(
                    "Windows.Storage.Provider.StorageProviderSyncRootManager",
                ))
            }
        },
        std::thread::sleep,
    )
    .map_err(|e| api_error("factory", e))?;
    // SAFETY: use the generated SDK vtable and output type. The live factory
    // owns the call; null-initialized output is converted only after success.
    let roots: windows_collections::IVectorView<
        windows::Storage::Provider::StorageProviderSyncRootInfo,
    > = unsafe {
        let mut output = null_mut();
        (factory.vtable().GetCurrentSyncRoots)(factory.as_raw(), &mut output)
            .and_then(|| windows::core::Type::from_abi(output))
    }
    .map_err(|e| api_error("inventory", e))?;
    let count = roots.Size().map_err(|e| api_error("count", e))?;
    if count > 128 {
        return Err(StorageError::InputLimit);
    }
    let ids = ancestors.iter().map(stamp).collect::<Result<Vec<_>, _>>()?;
    for i in 0..count {
        let root = roots.GetAt(i).map_err(|e| api_error("root", e))?;
        let folder = root.Path().map_err(|e| api_error("folder", e))?;
        let item: IStorageItem = folder.cast().map_err(|e| api_error("item", e))?;
        let path = item.Path().map_err(|e| api_error("path", e))?;
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
    if roots.Size().map_err(|e| api_error("recount", e))? != count {
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
#[path = "sync_diagnostics_tests.rs"]
mod diagnostics;

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
                    registered_sync_check(&[]).unwrap_or_else(|error| {
                        panic!(
                            "fresh thread inventory: {error:?}; {:?}",
                            diagnostics::last()
                        )
                    });
                }
            })
        })
        .collect();
    let mut completed = true;
    for child in children {
        completed &= child.join().is_ok();
    }
    assert!(completed, "all inventory threads must complete");
}

#[cfg(test)]
#[test]
fn native_activation_stopping_is_bounded_and_never_assumed_empty() {
    use windows::core::{Error, HRESULT};
    let mut calls = 0;
    let mut pauses = vec![];
    let result = activate_registered(
        || {
            calls += 1;
            if calls < 3 {
                Err(Error::from_hresult(HRESULT(CO_E_SERVER_STOPPING)))
            } else {
                Ok(42)
            }
        },
        |duration| pauses.push(duration),
    );
    assert_eq!(result.unwrap(), 42);
    assert_eq!(calls, 3);
    assert_eq!(pauses, vec![ACTIVATION_PAUSE; 2]);

    let mut calls = 0;
    let mut pauses = 0;
    let result: windows::core::Result<()> = activate_registered(
        || {
            calls += 1;
            Err(Error::from_hresult(HRESULT(CO_E_SERVER_STOPPING)))
        },
        |duration| {
            assert_eq!(duration, ACTIVATION_PAUSE);
            pauses += 1;
        },
    );
    assert_eq!(result.unwrap_err().code().0, CO_E_SERVER_STOPPING);
    assert_eq!(calls, ACTIVATION_ATTEMPTS);
    assert_eq!(pauses, ACTIVATION_ATTEMPTS - 1);
}

#[cfg(test)]
#[test]
fn native_activation_other_failures_are_immediate_and_preserved() {
    use windows::core::{Error, HRESULT};
    // Access denied, class missing, invalid function, changed apartment, unknown.
    for code in [
        0x80070005u32,
        0x80040154,
        0x80070001,
        0x80010106,
        0x8000ffff,
    ] {
        let mut calls = 0;
        let result: windows::core::Result<()> = activate_registered(
            || {
                calls += 1;
                Err(Error::from_hresult(HRESULT(code as i32)))
            },
            |_| panic!("only server-stopping permits a bounded pause"),
        );
        assert_eq!(result.unwrap_err().code().0, code as i32);
        assert_eq!(calls, 1);
    }
    assert_eq!(
        activate_registered(|| Ok(7), |_| panic!("no pause")).unwrap(),
        7
    );
}
