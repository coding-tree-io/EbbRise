//! Display size, for placing a break window that does not cover the whole screen.

use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Width and height of the primary display in physical pixels (the process is DPI aware
/// once the UI's event loop runs).
pub fn primary_screen_size() -> (i64, i64) {
    // SAFETY: no pointers; reads two system metrics.
    unsafe {
        (
            i64::from(GetSystemMetrics(SM_CXSCREEN)),
            i64::from(GetSystemMetrics(SM_CYSCREEN)),
        )
    }
}
