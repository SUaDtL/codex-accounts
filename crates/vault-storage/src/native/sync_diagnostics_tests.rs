//! Fixed API stage/HRESULT only. No raw OS text, paths or returned root data.
use std::cell::Cell;
thread_local! {
    // Copy-only diagnostic slot: no COM object, cleanup or TLS destructor.
    static LAST: Cell<Option<(&'static str, i32)>> = const { Cell::new(None) };
}
pub(super) fn record(stage: &'static str, code: i32) {
    LAST.set(Some((stage, code)));
}
pub(super) fn last() -> Option<(&'static str, i32)> {
    LAST.get()
}
