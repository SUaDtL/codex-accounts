//! The native dialog alone chooses the destination; the renderer supplies no path.
use crate::{OperationError, Vault};
use std::{
    ffi::OsString, fs::OpenOptions, io::Write, os::windows::ffi::OsStringExt, path::PathBuf,
};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetSaveFileNameW, OFN_EXPLORER, OFN_NOCHANGEDIR, OFN_NOREADONLYRETURN,
    OFN_NOTESTFILECREATE, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

impl Vault {
    /// Export only fixed application metadata and sanitized journal summaries.
    /// Cancellation returns false. Existing files are never overwritten.
    pub fn export_sanitized_diagnostics(&self) -> Result<bool, OperationError> {
        let bytes = self.storage.sanitized_diagnostics()?;
        let Some(path) = save_destination()? else {
            return Ok(false);
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    OperationError("E_DIAGNOSTICS_EXISTS")
                } else {
                    OperationError("E_DIAGNOSTICS_WRITE")
                }
            })?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| OperationError("E_DIAGNOSTICS_WRITE"))?;
        Ok(true)
    }
}

fn save_destination() -> Result<Option<PathBuf>, OperationError> {
    let mut filename = vec![0_u16; 32768];
    let default: Vec<u16> = "codex-accounts-diagnostics.json".encode_utf16().collect();
    filename[..default.len()].copy_from_slice(&default);
    let filter: Vec<u16> = "JSON files (*.json)\0*.json\0\0".encode_utf16().collect();
    let title: Vec<u16> = "Export sanitized diagnostics\0".encode_utf16().collect();
    let extension: Vec<u16> = "json\0".encode_utf16().collect();
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: filter.as_ptr(),
        nFilterIndex: 1,
        lpstrFile: filename.as_mut_ptr(),
        nMaxFile: filename.len() as u32,
        lpstrTitle: title.as_ptr(),
        lpstrDefExt: extension.as_ptr(),
        Flags: OFN_EXPLORER
            | OFN_NOCHANGEDIR
            | OFN_NOREADONLYRETURN
            | OFN_NOTESTFILECREATE
            | OFN_PATHMUSTEXIST,
        ..Default::default()
    };
    // All pointers refer to live bounded buffers for the synchronous dialog call.
    if unsafe { GetSaveFileNameW(&mut dialog) } == 0 {
        return if unsafe { CommDlgExtendedError() } == 0 {
            Ok(None)
        } else {
            Err(OperationError("E_DIAGNOSTICS_DIALOG"))
        };
    }
    let length = filename
        .iter()
        .position(|unit| *unit == 0)
        .filter(|length| *length > 0)
        .ok_or(OperationError("E_DIAGNOSTICS_DIALOG"))?;
    let path = PathBuf::from(OsString::from_wide(&filename[..length]));
    if !path.is_absolute() {
        return Err(OperationError("E_DIAGNOSTICS_DIALOG"));
    }
    Ok(Some(path))
}
