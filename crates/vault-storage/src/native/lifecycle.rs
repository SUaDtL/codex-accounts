//! Private native adapters, inside the existing unsafe boundary.
//! Public operations expose fixed actions, never path/process executors.
//! Synthetic child construction exists only in lifecycle_tests.rs.
#![deny(unsafe_op_in_unsafe_fn)]
mod consent;
mod diagnostics;
mod helper;
mod home;
mod launch;
pub(super) mod login;
mod operations;
mod process;
mod runtime_context;
mod usage;
mod usage_http;

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;

// Fixed file backend; arbitrary fixture construction is test-only.
mod target;
#[cfg(test)]
mod target_tests;

// Compare the helper's reported home with the already-retained native target.
// This is protocol spelling normalization, never filesystem identity validation.
fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    let normalize = |p: &std::path::Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_start_matches("\\\\?\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    normalize(a) == normalize(b)
}
