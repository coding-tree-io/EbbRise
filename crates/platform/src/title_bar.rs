//! A dark title bar for an app window, so it matches the window's dark content.

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Dwm::{
    DWMWA_CAPTION_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
use windows::core::HSTRING;

/// Gives the top-level window titled `title` a title bar in `rgb` (0xRRGGBB), with dark or
/// light caption text and buttons; returns whether a window was found. Use a title no other
/// app is likely to have. The colour needs Windows 11.
pub fn title_bar(title: &str, dark: bool, rgb: u32) -> bool {
    // SAFETY: the title is a valid wide string for the call.
    let Ok(window) = (unsafe { FindWindowW(None, &HSTRING::from(title)) }) else {
        return false;
    };
    let dark = i32::from(dark);
    // Windows wants 0x00BBGGRR.
    let colour = COLORREF(((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF));
    // SAFETY: each pointer is a live local of the size passed with it.
    unsafe {
        let _ = DwmSetWindowAttribute(
            window,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark).cast(),
            size_of::<i32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            window,
            DWMWA_CAPTION_COLOR,
            (&raw const colour).cast(),
            size_of::<COLORREF>() as u32,
        );
    }
    true
}
