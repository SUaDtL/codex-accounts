#![forbid(unsafe_code)]
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod metadata;
#[cfg(windows)]
mod preferences;
#[cfg(windows)]
mod vault;

#[cfg(windows)]
mod desktop {
    use super::{metadata, preferences, vault};
    use serde::Serialize;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::webview::{NewWindowResponse, PermissionKind, PermissionResponse};
    use tauri::{
        AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    };

    const TRAY_ID: &str = "accounts";
    const MAIN_WINDOW: &str = "main";
    const TRAY_OPEN: &str = "tray_open";
    const TRAY_QUIT: &str = "tray_quit";
    const TRAY_WINDOW: &str = "tray";
    const TRAY_WIDTH: f64 = 380.0;
    const TRAY_DEFAULT_HEIGHT: f64 = 270.0;
    const TRAY_MIN_HEIGHT: f64 = 174.0;
    const TRAY_MAX_HEIGHT: f64 = 594.0;

    #[derive(Debug, Eq, PartialEq)]
    enum TrayAction {
        Open,
        Quit,
        Ignore,
    }

    #[derive(Debug, Eq, PartialEq)]
    enum CloseAction {
        HideToTray,
        Quit,
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
        window: WebviewWindow,
        state: tauri::State<'_, InventoryState>,
    ) -> Result<metadata::Status, CommandError> {
        if window.label() != MAIN_WINDOW {
            return Err(CommandError::InventoryUnavailable);
        }
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
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_account_window(&window)?;
        let result = request_and_sync(&app, state.inner(), vault::Intent::List).await?;
        sync_main_from_tray(&window, &app, &result);
        Ok(result)
    }

    #[tauri::command]
    async fn open_vault(
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_account_window(&window)?;
        let result = request_and_sync(&app, state.inner(), vault::Intent::Open).await?;
        sync_main_from_tray(&window, &app, &result);
        Ok(result)
    }

    #[tauri::command]
    async fn unlock_vault(
        passphrase: String,
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(
            &app,
            state.inner(),
            vault::Intent::Unlock(zeroize::Zeroizing::new(passphrase)),
        )
        .await
    }

    #[tauri::command]
    async fn create_vault(
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::Create).await
    }

    #[tauri::command]
    async fn close_vault(
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::Close).await
    }

    #[tauri::command]
    async fn set_vault_passphrase(
        passphrase: String,
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(
            &app,
            state.inner(),
            vault::Intent::Harden(zeroize::Zeroizing::new(passphrase)),
        )
        .await
    }

    #[tauri::command]
    async fn remove_vault_passphrase(
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::Unharden).await
    }

    #[tauri::command]
    async fn open_codex(
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::OpenCodex).await
    }

    #[tauri::command]
    async fn capture_current(
        label: String,
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::Capture(label)).await
    }

    #[tauri::command]
    async fn add_account(
        label: String,
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        state.add_account(label).await
    }

    #[tauri::command]
    fn cancel_add_account(
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<(), &'static str> {
        require_main(&window)?;
        state.cancel_login()
    }

    #[tauri::command]
    fn read_add_account_status(
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<&'static str, &'static str> {
        require_main(&window)?;
        state.action_status()
    }

    #[tauri::command]
    async fn switch_profile(
        profile_id: String,
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_account_window(&window)?;
        let result = state.switch_profile(profile_id).await?;
        sync_main_from_tray(&window, &app, &result);
        Ok(result)
    }

    #[tauri::command]
    async fn reauthenticate_profile(
        profile_id: String,
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        state.reauthenticate(profile_id).await
    }

    #[tauri::command]
    fn cancel_switch(
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<(), &'static str> {
        require_main(&window)?;
        state.cancel_switch()
    }

    #[tauri::command]
    fn read_switch_status(
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<&'static str, &'static str> {
        require_main(&window)?;
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
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        state
            .request(vault::Intent::Recover(
                operation_id,
                matches!(choice, RecoveryChoice::Finish),
            ))
            .await
    }

    #[tauri::command]
    async fn recover_vault(
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::RecoverVault).await
    }

    #[tauri::command]
    async fn remove_profile(
        profile_id: String,
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::Remove(profile_id)).await
    }

    #[tauri::command]
    async fn rename_profile(
        profile_id: String,
        label: String,
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
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
        window: WebviewWindow,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_account_window(&window)?;
        state.request(vault::Intent::RefreshUsage(profile_id)).await
    }

    #[tauri::command]
    async fn export_sanitized_diagnostics(
        window: WebviewWindow,
        app: AppHandle,
        state: tauri::State<'_, vault::Worker>,
    ) -> Result<metadata::ProfileList, &'static str> {
        require_main(&window)?;
        request_and_sync(&app, state.inner(), vault::Intent::ExportDiagnostics).await
    }

    #[tauri::command]
    fn get_close_to_tray(
        window: WebviewWindow,
        state: tauri::State<'_, preferences::ClosePreference>,
    ) -> Result<bool, &'static str> {
        require_main(&window)?;
        Ok(state.enabled())
    }

    #[tauri::command]
    fn set_close_to_tray(
        enabled: bool,
        window: WebviewWindow,
        state: tauri::State<'_, preferences::ClosePreference>,
    ) -> Result<(), &'static str> {
        require_main(&window)?;
        state.set(enabled).map_err(|_| "preference_write_failed")
    }

    #[tauri::command]
    fn show_main_window(window: WebviewWindow, app: AppHandle) -> Result<(), &'static str> {
        require_tray(&window)?;
        hide_tray(&app);
        show_main(&app);
        Ok(())
    }

    #[tauri::command]
    fn hide_tray_window(window: WebviewWindow, app: AppHandle) -> Result<(), &'static str> {
        require_tray(&window)?;
        hide_tray(&app);
        Ok(())
    }

    #[tauri::command]
    fn resize_tray_window(
        rows: usize,
        paged: bool,
        caller: WebviewWindow,
        app: AppHandle,
    ) -> Result<(), &'static str> {
        require_tray(&caller)?;
        let Some(window) = app.get_webview_window(TRAY_WINDOW) else {
            return Err("window_unavailable");
        };
        let old_position = window.outer_position().ok();
        let old_size = window.outer_size().ok();
        let monitor = window.current_monitor().ok().flatten();
        let monitor_height = monitor.as_ref().map(|value| value.work_area().size.height);
        let scale = window.scale_factor().unwrap_or(1.0);
        let height = constrained_tray_height(rows, paged, monitor_height, scale);
        if window
            .set_size(tauri::LogicalSize::new(TRAY_WIDTH, height))
            .is_err()
        {
            return Err("window_unavailable");
        }
        let (Some(position), Some(size), Some(monitor)) = (old_position, old_size, monitor) else {
            return Ok(());
        };
        let area = monitor.work_area();
        let lower_half =
            position.y + size.height as i32 / 2 > area.position.y + area.size.height as i32 / 2;
        if !lower_half {
            return Ok(());
        }
        let new_height = window
            .outer_size()
            .map_or(size.height, |value| value.height);
        let min_y = area.position.y + 8;
        let max_y = area.position.y + area.size.height as i32 - new_height as i32 - 8;
        let y =
            (position.y + size.height as i32 - new_height as i32).clamp(min_y, max_y.max(min_y));
        let _ = window.set_position(tauri::PhysicalPosition::new(position.x, y));
        Ok(())
    }

    #[tauri::command]
    fn tray_page_capacity(caller: WebviewWindow) -> Result<usize, &'static str> {
        require_tray(&caller)?;
        let monitor_height = caller
            .current_monitor()
            .ok()
            .flatten()
            .map(|monitor| monitor.work_area().size.height);
        Ok(tray_page_capacity_for(
            monitor_height,
            caller.scale_factor().unwrap_or(1.0),
        ))
    }

    fn require_main(window: &WebviewWindow) -> Result<(), &'static str> {
        require_window(window, MAIN_WINDOW)
    }

    fn require_tray(window: &WebviewWindow) -> Result<(), &'static str> {
        require_window(window, TRAY_WINDOW)
    }

    fn require_account_window(window: &WebviewWindow) -> Result<(), &'static str> {
        if account_window_allowed(window.label()) {
            Ok(())
        } else {
            Err("window_not_allowed")
        }
    }

    fn require_window(window: &WebviewWindow, expected: &str) -> Result<(), &'static str> {
        if window_label_allowed(window.label(), expected) {
            Ok(())
        } else {
            Err("window_not_allowed")
        }
    }

    fn window_label_allowed(label: &str, expected: &str) -> bool {
        label == expected
    }

    fn account_window_allowed(label: &str) -> bool {
        matches!(label, MAIN_WINDOW | TRAY_WINDOW)
    }

    fn sync_main_from_tray(
        window: &WebviewWindow,
        app: &AppHandle,
        profiles: &metadata::ProfileList,
    ) {
        if window.label() == TRAY_WINDOW {
            let _ = app.emit_to(MAIN_WINDOW, "vault-state-changed", profiles);
        }
    }

    async fn request_and_sync(
        _app: &AppHandle,
        worker: &vault::Worker,
        intent: vault::Intent,
    ) -> Result<metadata::ProfileList, &'static str> {
        worker.request(intent).await
    }

    fn allowed_origin(url: &Url) -> bool {
        url.scheme() == "http"
            && url.host_str() == Some("tauri.localhost")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
    }

    fn allowed_main(url: &Url) -> bool {
        allowed_origin(url)
            && matches!(url.path(), "/" | "/index.html")
            && matches!(url.fragment(), None | Some("main"))
    }

    fn allowed_tray(url: &Url) -> bool {
        allowed_origin(url) && url.path() == "/tray.html" && url.fragment().is_none()
    }

    fn show_main(app: &AppHandle) {
        app.state::<vault::Worker>().cancel_hide();
        if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn hide_tray(app: &AppHandle) {
        if let Some(window) = app.get_webview_window(TRAY_WINDOW) {
            let _ = window.emit("tray-hidden", ());
            let _ = window.hide();
        }
    }

    fn popup_origin(
        click: (i32, i32),
        work_origin: (i32, i32),
        work_size: (u32, u32),
        popup_size: (u32, u32),
    ) -> (i32, i32) {
        let margin = 8;
        let min_x = work_origin.0 + margin;
        let max_x = work_origin.0 + work_size.0 as i32 - popup_size.0 as i32 - margin;
        let min_y = work_origin.1 + margin;
        let max_y = work_origin.1 + work_size.1 as i32 - popup_size.1 as i32 - margin;
        let x = (click.0 - popup_size.0 as i32 / 2).clamp(min_x, max_x.max(min_x));
        let candidate_y = if click.1 > work_origin.1 + work_size.1 as i32 / 2 {
            click.1 - popup_size.1 as i32 - 12
        } else {
            click.1 + 12
        };
        (x, candidate_y.clamp(min_y, max_y.max(min_y)))
    }

    fn monitor_contains(point: (i32, i32), position: (i32, i32), size: (u32, u32)) -> bool {
        point.0 >= position.0
            && point.0 < position.0 + size.0 as i32
            && point.1 >= position.1
            && point.1 < position.1 + size.1 as i32
    }

    fn tray_height(rows: usize, paged: bool) -> f64 {
        if rows == 0 {
            TRAY_MIN_HEIGHT
        } else {
            (78.0 + rows.min(5) as f64 * 96.0 + if paged { 36.0 } else { 0.0 })
                .clamp(TRAY_MIN_HEIGHT, TRAY_MAX_HEIGHT)
        }
    }

    fn constrained_tray_height(
        rows: usize,
        paged: bool,
        monitor_height: Option<u32>,
        scale: f64,
    ) -> f64 {
        let requested = tray_height(rows, paged);
        monitor_height.map_or(requested, |height| {
            let available = height as f64 / scale.max(1.0) - 16.0;
            requested.min(available.max(TRAY_MIN_HEIGHT))
        })
    }

    fn tray_page_capacity_for(monitor_height: Option<u32>, scale: f64) -> usize {
        monitor_height.map_or(5, |height| {
            let available = height as f64 / scale.max(1.0) - 16.0 - 78.0 - 36.0;
            (available / 96.0).floor().clamp(1.0, 5.0) as usize
        })
    }

    fn toggle_tray(app: &AppHandle, click: (i32, i32)) {
        let Some(window) = app.get_webview_window(TRAY_WINDOW) else {
            return;
        };
        if window.is_visible().unwrap_or(false) {
            hide_tray(app);
            return;
        }
        let popup_size = window
            .outer_size()
            .unwrap_or(tauri::PhysicalSize::new(380, 270));
        let monitor = window.available_monitors().ok().and_then(|monitors| {
            monitors.into_iter().find(|monitor| {
                let position = monitor.position();
                let size = monitor.size();
                monitor_contains(click, (position.x, position.y), (size.width, size.height))
            })
        });
        if let Some(monitor) = monitor {
            let area = monitor.work_area();
            let origin = popup_origin(
                click,
                (area.position.x, area.position.y),
                (area.size.width, area.size.height),
                (popup_size.width, popup_size.height),
            );
            let _ = window.set_position(tauri::PhysicalPosition::new(origin.0, origin.1));
        }
        let _ = window.emit("tray-opened", ());
        let _ = window.show();
        let _ = window.set_focus();
    }

    fn hide_main(app: &AppHandle) {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
            let _ = window.emit("window-hiding", ());
            let hidden = window.hide().is_ok();
            app.state::<vault::Worker>().cancel_hide();
            if hidden {
                let _ = window.emit("window-hidden", ());
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

    fn close_action(close_to_tray: bool) -> CloseAction {
        if close_to_tray {
            CloseAction::HideToTray
        } else {
            CloseAction::Quit
        }
    }

    fn tray_action(id: &str) -> TrayAction {
        match id {
            TRAY_OPEN => TrayAction::Open,
            TRAY_QUIT => TrayAction::Quit,
            _ => TrayAction::Ignore,
        }
    }

    fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
        let menu = Menu::new(app)?;
        let open = MenuItem::with_id(app, TRAY_OPEN, "Open", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, TRAY_QUIT, "Quit", true, None::<&str>)?;
        menu.append(&open)?;
        menu.append(&quit)?;
        Ok(menu)
    }

    fn install_tray(app: &AppHandle) -> tauri::Result<()> {
        let menu = tray_menu(app)?;
        let mut builder = TrayIconBuilder::with_id(TRAY_ID)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .tooltip("Codex Accounts")
            .on_menu_event(|app, event| match tray_action(event.id().as_ref()) {
                TrayAction::Open => show_main(app),
                TrayAction::Quit => request_quit(app),
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
                    if let TrayIconEvent::Click { position, .. } = event {
                        toggle_tray(
                            tray.app_handle(),
                            (position.x.round() as i32, position.y.round() as i32),
                        );
                    }
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
                if window.label() == TRAY_WINDOW {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            hide_tray(window.app_handle());
                        }
                        tauri::WindowEvent::Focused(false) => {
                            hide_tray(window.app_handle());
                        }
                        _ => {}
                    }
                    return;
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if close_action(window.state::<preferences::ClosePreference>().enabled())
                        == CloseAction::Quit
                    {
                        request_quit(window.app_handle());
                        return;
                    }
                    if window.state::<vault::Worker>().begin_hide() {
                        let app = window.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            match app
                                .state::<vault::Worker>()
                                .request(vault::Intent::Close)
                                .await
                            {
                                Ok(profiles) => {
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
                export_sanitized_diagnostics,
                get_close_to_tray,
                set_close_to_tray,
                show_main_window,
                hide_tray_window,
                resize_tray_window,
                tray_page_capacity
            ])
            .on_permission_request(|webview, kind| {
                if webview.label() == "main" && matches!(kind, PermissionKind::Notifications) {
                    PermissionResponse::Allow
                } else {
                    PermissionResponse::Deny
                }
            })
            .setup(|app| {
                app.manage(preferences::ClosePreference::load(app.handle()));
                WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("index.html".into()))
                    .title("Codex Accounts")
                    .inner_size(940.0, 360.0)
                    .min_inner_size(420.0, 320.0)
                    .theme(Some(tauri::Theme::Dark))
                    .use_https_scheme(false)
                    .devtools(false)
                    .on_navigation(allowed_main)
                    .on_new_window(|_, _| NewWindowResponse::Deny)
                    .build()?;
                WebviewWindowBuilder::new(app, TRAY_WINDOW, WebviewUrl::App("tray.html".into()))
                    .title("Codex Accounts")
                    .inner_size(TRAY_WIDTH, TRAY_DEFAULT_HEIGHT)
                    .min_inner_size(TRAY_WIDTH, TRAY_MIN_HEIGHT)
                    .max_inner_size(TRAY_WIDTH, TRAY_MAX_HEIGHT)
                    .resizable(false)
                    .decorations(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .shadow(true)
                    .visible(false)
                    .theme(Some(tauri::Theme::Dark))
                    .use_https_scheme(false)
                    .devtools(false)
                    .on_navigation(allowed_tray)
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
        fn navigation_stays_within_each_bundled_window() {
            for url in [
                "http://tauri.localhost/",
                "http://tauri.localhost/#main",
                "http://tauri.localhost/index.html",
                "http://tauri.localhost/index.html#main",
            ] {
                assert!(allowed_main(&Url::parse(url).unwrap()));
            }
            for url in [
                "https://example.invalid/",
                "http://tauri.localhost/other.html",
                "http://tauri.localhost/index.html?url=x",
                "http://tauri.localhost/?url=x",
                "file:///C:/SYNTHETIC",
                "http://user@tauri.localhost/index.html",
            ] {
                assert!(!allowed_main(&Url::parse(url).unwrap()));
            }
            let tray = Url::parse("http://tauri.localhost/tray.html").unwrap();
            assert!(allowed_tray(&tray));
            assert!(!allowed_main(&tray));
            assert!(!allowed_tray(
                &Url::parse("http://tauri.localhost/index.html").unwrap()
            ));
        }

        #[test]
        fn close_preference_routes_to_hide_or_quit() {
            assert_eq!(close_action(true), CloseAction::HideToTray);
            assert_eq!(close_action(false), CloseAction::Quit);
        }

        #[test]
        fn renderer_authority_is_scoped_to_the_calling_window() {
            assert!(window_label_allowed(MAIN_WINDOW, MAIN_WINDOW));
            assert!(!window_label_allowed(TRAY_WINDOW, MAIN_WINDOW));
            assert!(account_window_allowed(MAIN_WINDOW));
            assert!(account_window_allowed(TRAY_WINDOW));
            assert!(!account_window_allowed("unexpected"));
        }

        #[test]
        fn tray_menu_dispatches_only_window_and_quit_actions() {
            assert_eq!(tray_action(TRAY_OPEN), TrayAction::Open);
            assert_eq!(tray_action(TRAY_QUIT), TrayAction::Quit);
            assert_eq!(tray_action("tray_profile_other-id"), TrayAction::Ignore);
            assert_eq!(tray_action("unknown"), TrayAction::Ignore);
        }

        #[test]
        fn popup_origin_stays_inside_the_monitor_work_area() {
            assert_eq!(
                popup_origin((1850, 1030), (0, 0), (1920, 1040), (380, 300)),
                (1532, 718)
            );
            assert_eq!(
                popup_origin((10, 10), (0, 0), (1920, 1040), (380, 300)),
                (8, 22)
            );
        }

        #[test]
        fn tray_click_monitor_selection_includes_taskbars_and_secondary_monitors() {
            assert!(monitor_contains((1850, 1070), (0, 0), (1920, 1080)));
            assert!(monitor_contains((-1200, 900), (-1280, 0), (1280, 1024)));
            assert!(!monitor_contains((-1, 500), (0, 0), (1920, 1080)));
        }

        #[test]
        fn tray_height_fits_common_account_counts_and_caps_large_lists() {
            assert_eq!(tray_height(0, false), 174.0);
            assert_eq!(tray_height(1, false), 174.0);
            assert_eq!(tray_height(2, false), 270.0);
            assert_eq!(tray_height(5, false), 558.0);
            assert_eq!(tray_height(5, true), 594.0);
            assert_eq!(tray_height(1, true), 210.0);
            assert_eq!(tray_page_capacity_for(Some(1040), 2.0), 4);
            assert_eq!(constrained_tray_height(5, true, Some(1040), 2.0), 504.0);
            assert_eq!(tray_page_capacity_for(Some(1040), 1.0), 5);
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
