//! Windows toast notifications from an unpackaged exe (ADR 0001, R2).

use tauri_winrt_notification::Toast;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};
use windows::core::HSTRING;

/// Registers `app_id` as an AppUserModelID under HKCU so toasts sent under it are shown.
///
/// Call it at every start: the probe showed that the first toast right after registration
/// is not displayed.
///
/// # Errors
/// When the registry value cannot be written.
pub fn register(app_id: &str, display_name: &str) -> windows::core::Result<()> {
    let key = HSTRING::from(format!(r"Software\Classes\AppUserModelId\{app_id}"));
    let value: Vec<u16> = display_name.encode_utf16().chain([0]).collect();
    // SAFETY: `value` is a null-terminated UTF-16 buffer that outlives the call, and its byte
    // length is passed with it.
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &key,
            &HSTRING::from("DisplayName"),
            REG_SZ.0,
            Some(value.as_ptr().cast()),
            (value.len() * size_of::<u16>()) as u32,
        )
        .ok()
    }
}

/// Shows a toast under `app_id` on a new thread and returns at once; errors are dropped.
///
/// Showing can block for seconds, which would freeze the UI thread.
pub fn show(app_id: &str, title: &str, text: &str, silent: bool) {
    let (app_id, title, text) = (app_id.to_owned(), title.to_owned(), text.to_owned());
    std::thread::spawn(move || {
        let mut toast = Toast::new(&app_id).title(&title).text1(&text);
        if silent {
            toast = toast.sound(None);
        }
        let _ = toast.show();
    });
}
