#![forbid(unsafe_code)]

#[cfg(windows)]
fn main() {
    tauri_build::build();
}
#[cfg(not(windows))]
fn main() {}
