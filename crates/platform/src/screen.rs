//! Display size, for placing a break window that does not cover the whole screen, and the
//! small icon size, for the tray icon.

use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXSCREEN, SM_CXSMICON, SM_CYSCREEN,
};

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

/// Width of a small icon, such as a tray icon, in physical pixels: 16 at 100% scaling.
pub fn small_icon_size() -> u32 {
    // SAFETY: no pointers; reads one system metric.
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) };
    u32::try_from(size).unwrap_or(16).max(16)
}
