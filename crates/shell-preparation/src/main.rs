#![forbid(unsafe_code)]
#[cfg(windows)]
mod preview {
    use std::os::windows::fs::MetadataExt;
    use std::{
        io,
        path::{Path, PathBuf},
    };
    use tauri::webview::NewWindowResponse;
    use tauri::{Url, WebviewUrl, WebviewWindowBuilder};

    pub(super) fn allowed(url: &Url) -> bool {
        url.scheme() == "http"
            && url.host_str() == Some("tauri.localhost")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && matches!(
                url.path(),
                "/accounts.html" | "/status.html" | "/recovery.html"
            )
            && matches!(url.fragment(), None | Some("main"))
    }
    fn ordinary_directory(path: &Path) -> io::Result<()> {
        for ancestor in path.ancestors() {
            let metadata = ancestor.symlink_metadata()?;
            if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
                return Err(io::Error::other("preview_directory_refused"));
            }
        }
        Ok(())
    }
    // Internal developer launcher input; no renderer path or account CLI.
    fn owned_profile() -> io::Result<PathBuf> {
        let run = PathBuf::from(
            std::env::var_os("CODEX_ACCOUNTS_PREVIEW_RUN")
                .ok_or_else(|| io::Error::other("preview_run_required"))?,
        );
        let valid_name = run
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix("preview-launch180-"))
            .is_some_and(|suffix| {
                suffix.len() == 32 && suffix.bytes().all(|b| b.is_ascii_hexdigit())
            });
        if !run.is_absolute() || !valid_name {
            return Err(io::Error::other("preview_run_refused"));
        }
        ordinary_directory(&run)?;
        let profile = run.join("webview-data");
        std::fs::create_dir(&profile)?; // Exclusive; never reuse or fall back.
        ordinary_directory(&profile)?;
        Ok(profile)
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
        // Query before Builder. No installer, downloader or browser opener.
        tauri::webview_version().map_err(|_| ())?;
        let profile = owned_profile().map_err(|_| ())?;
        tauri::Builder::default()
            .setup(move |app| {
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("accounts.html".into()))
                    .title("Codex Accounts Static Preview - Unqualified")
                    .use_https_scheme(false)
                    .disable_javascript()
                    .data_directory(profile.clone())
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
        fn only_bundled_preview_destinations_and_skip_fragments_navigate() {
            for page in ["accounts.html", "status.html", "recovery.html"] {
                for fragment in ["", "#main"] {
                    assert!(allowed(
                        &Url::parse(&format!("http://tauri.localhost/{page}{fragment}")).unwrap()
                    ));
                }
            }
            assert!(allowed(
                &Url::parse("http://tauri.localhost:80/accounts.html").unwrap()
            ));
            for address in [
                "https://tauri.localhost/accounts.html",
                "http://evil.invalid/accounts.html",
                "http://tauri.localhost.evil.invalid/accounts.html",
                "http://tauri.localhost:81/accounts.html",
                "http://user@tauri.localhost/accounts.html",
                "http://tauri.localhost/accounts.html?x=1",
                "http://tauri.localhost/accounts.html#other",
                "http://tauri.localhost/%61ccounts.html",
                "http://tauri.localhost/preview.css",
                "http://tauri.localhost/secret.json",
                "file:///C:/SYNTHETIC",
                "data:text/html,SYNTHETIC",
                "javascript:alert(1)",
                "about:blank",
            ] {
                assert!(!allowed(&Url::parse(address).unwrap()));
            }
        }
    }
}
fn main() {
    #[cfg(windows)]
    if preview::run().is_ok() {
        return;
    }
    eprintln!("static_preview_refused");
    std::process::exit(1);
}
