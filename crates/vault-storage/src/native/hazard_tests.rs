//! Actual reparse-point test in exclusively created synthetic directories.
use super::*;

#[test]
fn native_junction_root_and_ancestor_refuse_without_touching_target() {
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_REPARSE_POINT;
    use windows_sys::Win32::System::SystemServices::IO_REPARSE_TAG_MOUNT_POINT;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    let fixture = super::super::super::tests::Sandbox::new();
    let target = fixture.0.with_extension("target");
    let security = Security::new().unwrap();
    for path in [&fixture.0, &target] {
        assert_ne!(
            unsafe { CreateDirectoryW(wide(path).unwrap().as_ptr(), &security.attrs()) },
            0
        );
    }
    std::fs::write(target.join("synthetic.txt"), b"SYNTHETIC_UNCHANGED").unwrap();
    let junction = open_file(&fixture.0, GENERIC_WRITE, 0, OPEN_EXISTING, None).unwrap();
    let substitute: Vec<u16> = format!("\\??\\{}", target.display())
        .encode_utf16()
        .collect();
    let print: Vec<u16> = target.as_os_str().encode_wide().collect();
    let sub_len = (substitute.len() * 2) as u16;
    let print_len = (print.len() * 2) as u16;
    let data_len = 8 + sub_len + 2 + print_len + 2;
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
    buffer.extend_from_slice(&data_len.to_le_bytes());
    buffer.extend_from_slice(&0u16.to_le_bytes());
    for value in [0, sub_len, sub_len + 2, print_len] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in substitute.into_iter().chain([0]).chain(print).chain([0]) {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let mut returned = 0;
    // SAFETY: documented bounded mount-point layout; only our new empty test
    // directory is changed. This helper is not compiled into product code.
    assert_ne!(
        unsafe {
            DeviceIoControl(
                junction.as_raw_handle(),
                FSCTL_SET_REPARSE_POINT,
                buffer.as_ptr().cast(),
                buffer.len() as u32,
                null_mut(),
                0,
                &mut returned,
                null_mut(),
            )
        },
        0
    );
    drop(junction);
    let root_result = Disk::at(&fixture.0, false);
    let child_result = Disk::at(&fixture.0.join("vault"), true);
    std::fs::remove_dir(&fixture.0).unwrap();
    assert!(matches!(root_result, Err(StorageError::UnsafePath)));
    assert!(matches!(child_result, Err(StorageError::UnsafePath)));
    assert!(!target.join("vault").exists());
    assert_eq!(
        std::fs::read(target.join("synthetic.txt")).unwrap(),
        b"SYNTHETIC_UNCHANGED"
    );
}
