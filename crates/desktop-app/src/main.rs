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
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::webview::NewWindowResponse;
    use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

    const TRAY_ID: &str = "accounts";
    const TRAY_OPEN: &str = "tray_open";
    const TRAY_QUIT: &str = "tray_quit";
    const TRAY_PROFILE: &str = "tray_profile_";

    #[derive(Debug, Eq, PartialEq)]
    enum TrayAction<'a> {
        Open,
        Quit,
        Profile(&'a str),
        Ignore,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TrayProfileItem {
        command: String,
        label: String,
        enabled: bool,
        checked: bool,
    }

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
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::List).await
    }

    #[tauri::command]
    async fn open_vault(
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::Open).await
    }

    #[tauri::command]
    async fn unlock_vault(
        passphrase: String,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(
            &app,
            state.inner(),
            vault::Intent::Unlock(zeroize::Zeroizing::new(passphrase)),
        )
        .await
    }

    #[tauri::command]
    async fn create_vault(
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::Create).await
    }

    #[tauri::command]
    async fn close_vault(
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::Close).await
    }

    #[tauri::command]
    async fn set_vault_passphrase(
        passphrase: String,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(
            &app,
            state.inner(),
            vault::Intent::Harden(zeroize::Zeroizing::new(passphrase)),
        )
        .await
    }

    #[tauri::command]
    async fn remove_vault_passphrase(
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::Unharden).await
    }

    #[tauri::command]
    async fn open_codex(
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::OpenCodex).await
    }

    #[tauri::command]
    async fn capture_current(
        label: String,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::Capture(label)).await
    }

    #[tauri::command]
    async fn add_account(
        label: String,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        let result = state.add_account(label).await?;
        let _ = update_tray_menu(&app, &result);
        Ok(result)
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
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        let result = state.switch_profile(profile_id).await?;
        let _ = update_tray_menu(&app, &result);
        Ok(result)
    }

    #[tauri::command]
    async fn reauthenticate_profile(
        profile_id: String,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        let result = state.reauthenticate(profile_id).await?;
        let _ = update_tray_menu(&app, &result);
        Ok(result)
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
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        let result = state
            .request(vault::Intent::Recover(
                operation_id,
                matches!(choice, RecoveryChoice::Finish),
            ))
            .await?;
        let _ = update_tray_menu(&app, &result);
        Ok(result)
    }

    #[tauri::command]
    async fn recover_vault(
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::RecoverVault).await
    }

    #[tauri::command]
    async fn remove_profile(
        profile_id: String,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::Remove(profile_id)).await
    }

    #[tauri::command]
    async fn rename_profile(
        profile_id: String,
        label: String,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(
            &app,
            state.inner(),
            vault::Intent::Rename(profile_id, label),
        )
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
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        request_and_sync(&app, state.inner(), vault::Intent::ExportDiagnostics).await
    }

    async fn request_and_sync(
        app: &AppHandle,
        worker: &vault::Worker,
        intent: vault::Intent,
    ) -> Result<metadata::ProfileList, &'static str> {
        let result = worker.request(intent).await?;
        // A completed vault action remains successful even if Explorer refuses
        // a tray-menu refresh. The main window still reflects the result.
        let _ = update_tray_menu(app, &result);
        Ok(result)
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

    fn show_main(app: &AppHandle) {
        app.state::<vault::Worker>().cancel_hide();
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn hide_main(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("window-hiding", ());
            if window.hide().is_err() {
                app.state::<vault::Worker>().cancel_hide();
            }
        } else {
            app.state::<vault::Worker>().cancel_hide();
        }
    }

    fn request_quit(app: &AppHandle) {
        if app.state::<vault::Worker>().cancel_for_close() {
            show_main(app);
        } else {
            app.exit(0);
        }
    }

    fn tray_action(id: &str) -> TrayAction<'_> {
        match id {
            TRAY_OPEN => TrayAction::Open,
            TRAY_QUIT => TrayAction::Quit,
            _ => id
                .strip_prefix(TRAY_PROFILE)
                .filter(|profile| !profile.is_empty())
                .map_or(TrayAction::Ignore, TrayAction::Profile),
        }
    }

    fn tray_profile_items(profiles: &metadata::ProfileList) -> Vec<TrayProfileItem> {
        if !matches!(profiles.state, metadata::VaultState::Open) {
            return Vec::new();
        }
        profiles
            .profiles
            .iter()
            .map(|profile| {
                let checked = profiles.last_known_active_profile.as_deref() == Some(&profile.id);
                TrayProfileItem {
                    command: format!("{TRAY_PROFILE}{}", profile.id),
                    label: profile.label.replace('&', "&&"),
                    enabled: !checked,
                    checked,
                }
            })
            .collect()
    }

    fn tray_menu(
        app: &AppHandle,
        profiles: Option<&metadata::ProfileList>,
    ) -> tauri::Result<Menu<tauri::Wry>> {
        let menu = Menu::new(app)?;
        if let Some(profiles) = profiles {
            let items = tray_profile_items(profiles);
            for item in &items {
                let item = CheckMenuItem::with_id(
                    app,
                    &item.command,
                    &item.label,
                    item.enabled,
                    item.checked,
                    None::<&str>,
                )?;
                menu.append(&item)?;
            }
            if !items.is_empty() {
                menu.append(&PredefinedMenuItem::separator(app)?)?;
            }
        }
        let open = MenuItem::with_id(app, TRAY_OPEN, "Open", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, TRAY_QUIT, "Quit", true, None::<&str>)?;
        menu.append(&open)?;
        menu.append(&quit)?;
        Ok(menu)
    }

    fn update_tray_menu(app: &AppHandle, profiles: &metadata::ProfileList) -> tauri::Result<()> {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            tray.set_menu(Some(tray_menu(app, Some(profiles))?))?;
        }
        Ok(())
    }

    fn install_tray(app: &AppHandle) -> tauri::Result<()> {
        let menu = tray_menu(app, None)?;
        let mut builder = TrayIconBuilder::with_id(TRAY_ID)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .tooltip("Codex Accounts")
            .on_menu_event(|app, event| match tray_action(event.id().as_ref()) {
                TrayAction::Open => show_main(app),
                TrayAction::Quit => request_quit(app),
                TrayAction::Profile(profile) => {
                    show_main(app);
                    let _ = app.emit_to("main", "tray-switch-profile", profile);
                }
                TrayAction::Ignore => {}
            })
            .on_tray_icon_event(|tray, event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                ) {
                    show_main(tray.app_handle());
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        builder.build(app)?;
        Ok(())
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
                    api.prevent_close();
                    if window.state::<vault::Worker>().begin_hide() {
                        let app = window.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            match app
                                .state::<vault::Worker>()
                                .request(vault::Intent::Close)
                                .await
                            {
                                Ok(profiles) => {
                                    let _ = update_tray_menu(&app, &profiles);
                                    let _ = app.emit_to("main", "vault-state-changed", &profiles);
                                    hide_main(&app);
                                }
                                Err(error) => {
                                    app.state::<vault::Worker>().cancel_hide();
                                    let _ = app.emit_to("main", "vault-lock-failed", error);
                                }
                            }
                        });
                    } else {
                        hide_main(window.app_handle());
                    }
                }
            })
            .invoke_handler(tauri::generate_handler![
                read_status,
                list_profiles,
                open_vault,
                unlock_vault,
                create_vault,
                close_vault,
                set_vault_passphrase,
                remove_vault_passphrase,
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
                install_tray(app.handle())?;
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

        #[test]
        fn tray_model_marks_only_the_active_profile_and_dispatches_exact_ids() {
            let mut profiles = metadata::unopened_profiles(Some(true));
            profiles.state = metadata::VaultState::Open;
            profiles.last_known_active_profile = Some("active-id".into());
            profiles.profiles = vec![
                metadata::Profile {
                    id: "active-id".into(),
                    label: "Personal & Work".into(),
                    domain: "ChatGPT".into(),
                    generation: "generation-a".into(),
                    captured_at: 1,
                },
                metadata::Profile {
                    id: "other-id".into(),
                    label: "Other".into(),
                    domain: "ChatGPT".into(),
                    generation: "generation-b".into(),
                    captured_at: 2,
                },
            ];
            assert_eq!(
                tray_profile_items(&profiles),
                vec![
                    TrayProfileItem {
                        command: "tray_profile_active-id".into(),
                        label: "Personal && Work".into(),
                        enabled: false,
                        checked: true,
                    },
                    TrayProfileItem {
                        command: "tray_profile_other-id".into(),
                        label: "Other".into(),
                        enabled: true,
                        checked: false,
                    },
                ]
            );
            assert_eq!(tray_action(TRAY_OPEN), TrayAction::Open);
            assert_eq!(tray_action(TRAY_QUIT), TrayAction::Quit);
            assert_eq!(
                tray_action("tray_profile_other-id"),
                TrayAction::Profile("other-id")
            );
            assert_eq!(tray_action("tray_profile_"), TrayAction::Ignore);
            assert_eq!(tray_action("unknown"), TrayAction::Ignore);
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
