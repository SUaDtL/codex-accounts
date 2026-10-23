//! Native account confirmation uses labels resolved from the authenticated vault.
//! No renderer-supplied consent flag or preview label grants mutation authority.
use crate::journal::Failure;
use crate::StorageError;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_SETFOREGROUND, MB_TASKMODAL, MB_YESNO,
};

fn confirm(message: String) -> Result<(), Failure> {
    let message: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Codex Accounts\0".encode_utf16().collect();
    // SAFETY: owned NUL-terminated strings survive the synchronous native dialog.
    // The vault worker keeps its operation lock while the user makes the choice.
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION | MB_SETFOREGROUND | MB_TASKMODAL,
        )
    };
    if result == IDYES {
        Ok(())
    } else {
        Err(Failure::Cancelled)
    }
}

pub(super) fn switch(source: &str, target: &str) -> Result<(), Failure> {
    confirm(format!(
        "Switch from “{source}” to “{target}”?\n\n\
         The current account will be saved before the selected account is installed. \
         The official runtime may refresh sign-in credentials.\n\n\
         Codex will reopen and may restore its windows normally. Existing projects \
         and conversation history stay on this computer. Accounts does not send \
         prompts or resume work.\n\nContinue?"
    ))
}

pub(super) fn capture(label: &str) -> Result<(), Failure> {
    confirm(format!(
        "Save the current Codex account as “{label}”?\n\nIts sign-in credentials will be copied into the protected account vault. The account installed in Codex will not change.\n\nContinue?"
    ))
}

pub(super) fn vault_hardening(enable: bool) -> Result<(), StorageError> {
    let message = if enable {
        "Add passphrase protection to this vault?\n\nYou will need the passphrase each time you open it. Losing the passphrase permanently locks the saved accounts.\n\nContinue?"
    } else {
        "Remove passphrase protection from this vault?\n\nThe vault will remain encrypted and protected for your Windows user. It will open without a passphrase.\n\nContinue?"
    };
    confirm(message.to_owned()).map_err(|_| StorageError::Cancelled)
}

pub(super) fn recover(source: &str, target: &str, finish: bool) -> Result<(), Failure> {
    let action = if finish {
        format!("Finish the switch from “{source}” to “{target}”?")
    } else {
        format!("Restore “{source}” from the interrupted switch to “{target}”?")
    };
    confirm(format!(
        "{action}\n\nRecovery checks the encrypted journal and current files before \
         making changes. Unrecognized changes will not be overwritten. \
         Finishing may check credentials and reopen Codex; restoration uses the \
         recorded source.\n\nContinue?"
    ))
}

pub(super) fn reauthenticate(label: &str) -> Result<(), Failure> {
    confirm(format!(
        "Sign in again to “{label}”?\n\nThe official sign-in will open in your browser. \
         Sign in to this same account and workspace. A different account will be \
         refused without replacing the saved profile.\n\nIf this is your current \
         Desktop account, its sign-in credentials will be updated and Codex will \
         reopen. Signing in again to another saved account updates that saved \
         account only.\n\nContinue?"
    ))
}
