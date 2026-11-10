//! Display size, for placing a break window that does not cover the whole screen, and the
//! small icon size, for the tray icon.

use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, MONITORINFOF_PRIMARY, SM_CXSCREEN, SM_CXSMICON, SM_CYSCREEN,
};
use windows::core::BOOL;

/// A rectangle in physical pixels, relative to the virtual desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i64,
    /// Top edge.
    pub y: i64,
    /// Width.
    pub width: i64,
    /// Height.
    pub height: i64,
}

impl From<RECT> for Rect {
    fn from(rect: RECT) -> Self {
        Self {
            x: rect.left.into(),
            y: rect.top.into(),
            width: i64::from(rect.right) - i64::from(rect.left),
            height: i64::from(rect.bottom) - i64::from(rect.top),
        }
    }
}

/// A connected display and its available work area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    /// Monitor handle, valid until the display configuration changes.
    pub id: i64,
    /// Full display bounds.
    pub bounds: Rect,
    /// Bounds excluding taskbars and other reserved areas.
    pub work_area: Rect,
}

/// Display inventory and selected display ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Displays {
    /// Primary first, then ordered by left and top position.
    pub displays: Vec<Display>,
    /// Primary display id.
    pub primary: i64,
    /// Display id under the cursor.
    pub cursor: i64,
}

/// Lists connected displays, their work areas, and the primary and cursor displays.
///
/// # Errors
/// Returns a Windows error if enumeration, monitor information, or cursor lookup fails.
pub fn displays() -> windows::core::Result<Displays> {
    // SAFETY: the context is a documented constant, and the previous context is restored below.
    let previous =
        unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if previous.0.is_null() {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(0x80004005u32 as i32),
            "DPI awareness unavailable",
        ));
    }
    let _restore = DpiContext(previous);
    let mut found = Vec::<(Display, bool)>::new();
    // SAFETY: the callback receives a pointer to `found`, which lives until enumeration returns.
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect_display),
            LPARAM((&raw mut found) as isize),
        )
    }
    .ok()?;
    let primary = found
        .iter()
        .find(|(_, primary)| *primary)
        .map(|(display, _)| display.id)
        .ok_or_else(|| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "primary display unavailable",
            )
        })?;
    let mut point = POINT::default();
    // SAFETY: `point` is a valid output pointer.
    unsafe { GetCursorPos(&mut point)? };
    // SAFETY: the point is a value and the function returns a monitor handle.
    let cursor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) }.0 as isize as i64;
    found.sort_by_key(|(display, primary)| (!primary, display.bounds.x, display.bounds.y));
    Ok(Displays {
        displays: found.into_iter().map(|(display, _)| display).collect(),
        primary,
        cursor,
    })
}

struct DpiContext(DPI_AWARENESS_CONTEXT);
impl Drop for DpiContext {
    fn drop(&mut self) {
        // SAFETY: this was the valid context of the current thread before enumeration.
        unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}

unsafe extern "system" fn collect_display(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is initialized with the required size and `monitor` comes from enumeration.
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return BOOL(0);
    }
    // SAFETY: `data` is the exclusive vector passed to the synchronous enumeration.
    let displays = unsafe { &mut *(data.0 as *mut Vec<(Display, bool)>) };
    displays.push((
        Display {
            id: monitor.0 as isize as i64,
            bounds: info.rcMonitor.into(),
            work_area: info.rcWork.into(),
        },
        info.dwFlags & MONITORINFOF_PRIMARY != 0,
    ));
    BOOL(1)
}

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
