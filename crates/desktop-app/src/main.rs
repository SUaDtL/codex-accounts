#![forbid(unsafe_code)]
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod metadata;
#[cfg(windows)]
mod vault;

#[cfg(windows)]
mod desktop {
    use super::{metadata, vault};
    use serde::Serialize;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tauri::webview::NewWindowResponse;
    use tauri::{Manager, Url, WebviewUrl, WebviewWindowBuilder};

    #[derive(Default)]
    struct InventoryState(Arc<AtomicBool>);

    // One inventory at a time, including invocations outside normal UI flow.
    struct InventoryGuard(Arc<AtomicBool>);
    impl Drop for InventoryGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum CommandError {
        InventoryBusy,
        InventoryUnavailable,
    }

    #[tauri::command]
    async fn read_status(
        state: tauri::State<'_, InventoryState>,
    ) -> Result<metadata::Status, CommandError> {
        state
            .inner()
            .0
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| CommandError::InventoryBusy)?;
        let guard = InventoryGuard(state.inner().0.clone());
        tauri::async_runtime::spawn_blocking(move || {
            let _guard = guard;
            metadata::inspect()
        })
        .await
        .map_err(|_| CommandError::InventoryUnavailable)
    }

    #[tauri::command]
    async fn list_profiles(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::List).await
    }

    #[tauri::command]
    async fn open_vault(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::Open).await
    }

    #[tauri::command]
    async fn create_vault(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::Create).await
    }

    #[tauri::command]
    async fn close_vault(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::Close).await
    }

    #[tauri::command]
    async fn open_codex(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::OpenCodex).await
    }

    #[tauri::command]
    async fn capture_current(
        label: String,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::Capture(label)).await
    }

    #[tauri::command]
    async fn add_account(
        label: String,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.add_account(label).await
    }

    #[tauri::command]
    fn cancel_add_account(state: tauri::State<'_, vault::Worker>) -> Result<(), &'static str> {
        state.cancel_login()
    }

    #[tauri::command]
    fn read_add_account_status(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<&'static str, &'static str> {
        state.action_status()
    }

    #[tauri::command]
    async fn switch_profile(
        profile_id: String,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.switch_profile(profile_id).await
    }

    #[tauri::command]
    async fn reauthenticate_profile(
        profile_id: String,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.reauthenticate(profile_id).await
    }

    #[tauri::command]
    fn cancel_switch(state: tauri::State<'_, vault::Worker>) -> Result<(), &'static str> {
        state.cancel_switch()
    }

    #[tauri::command]
    fn read_switch_status(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<&'static str, &'static str> {
        state.action_status()
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum RecoveryChoice {
        Restore,
        Finish,
    }

    #[tauri::command]
    async fn recover_switch(
        operation_id: String,
        choice: RecoveryChoice,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state
            .request(vault::Intent::Recover(
                operation_id,
                matches!(choice, RecoveryChoice::Finish),
            ))
            .await
    }

    #[tauri::command]
    async fn recover_vault(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::RecoverVault).await
    }

    #[tauri::command]
    async fn remove_profile(
        profile_id: String,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::Remove(profile_id)).await
    }

    #[tauri::command]
    async fn rename_profile(
        profile_id: String,
        label: String,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state
            .request(vault::Intent::Rename(profile_id, label))
            .await
    }

    #[tauri::command]
    async fn refresh_usage(
        profile_id: String,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::RefreshUsage(profile_id)).await
    }

    #[tauri::command]
    async fn export_sanitized_diagnostics(
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        state.request(vault::Intent::ExportDiagnostics).await
    }

    fn allowed(url: &Url) -> bool {
        url.scheme() == "http"
            && url.host_str() == Some("tauri.localhost")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            // Tauri normalizes App("index.html") to its bundled root URL.
            && matches!(url.path(), "/" | "/index.html")
            && matches!(url.fragment(), None | Some("main"))
    }

    pub(super) fn run() -> Result<(), ()> {
        if std::env::args_os().len() != 1
            || std::env::vars_os().any(|(name, _)| {
                let name = name.to_string_lossy().to_ascii_uppercase();
                name.starts_with("WEBVIEW2_") || name == "TAURI_WEBVIEW_AUTOMATION"
            })
        {
            return Err(());
        }
        tauri::webview_version().map_err(|_| ())?;
        tauri::Builder::default()
            .manage(InventoryState::default())
            .manage(vault::Worker::start().map_err(|_| ())?)
            .on_window_event(|window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if window.state::<vault::Worker>().cancel_for_close() {
                        api.prevent_close();
                    }
                }
            })
            .invoke_handler(tauri::generate_handler![
                read_status,
                list_profiles,
                open_vault,
                create_vault,
                close_vault,
                open_codex,
                capture_current,
                add_account,
                cancel_add_account,
                read_add_account_status,
                switch_profile,
                reauthenticate_profile,
                cancel_switch,
                read_switch_status,
                recover_switch,
                recover_vault,
                remove_profile,
                rename_profile,
                refresh_usage,
                export_sanitized_diagnostics
            ])
            .setup(|app| {
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("Codex Accounts")
                    .inner_size(940.0, 360.0)
                    .min_inner_size(420.0, 320.0)
                    .theme(Some(tauri::Theme::Dark))
                    .use_https_scheme(false)
                    .devtools(false)
                    .on_navigation(allowed)
                    .on_new_window(|_, _| NewWindowResponse::Deny)
                    .build()?;
                Ok(())
            })
            .run(tauri::generate_context!())
            .map_err(|_| ())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn navigation_stays_on_bundled_application() {
            for url in [
                "http://tauri.localhost/",
                "http://tauri.localhost/#main",
                "http://tauri.localhost/index.html",
                "http://tauri.localhost/index.html#main",
            ] {
                assert!(allowed(&Url::parse(url).unwrap()));
            }
            for url in [
                "https://example.invalid/",
                "http://tauri.localhost/other.html",
                "http://tauri.localhost/index.html?url=x",
                "http://tauri.localhost/?url=x",
                "file:///C:/SYNTHETIC",
                "http://user@tauri.localhost/index.html",
            ] {
                assert!(!allowed(&Url::parse(url).unwrap()));
            }
        }
    }
}

fn main() {
    #[cfg(windows)]
    if desktop::run().is_ok() {
        return;
    }
    eprintln!("codex_accounts_startup_unavailable");
    std::process::exit(1);
}
