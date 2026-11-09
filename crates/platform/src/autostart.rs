//! Start at login through the current user's `Run` registry key, as Electron's
//! `setLoginItemSettings` does on Windows.

use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::{HSTRING, PCWSTR, w};

const RUN: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

/// The command line Windows runs at login for `name`; `None` when there is none.
pub fn autostart_command(name: &str) -> Option<String> {
    let name = HSTRING::from(name);
    let mut buffer = [0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: the buffer and its byte size match; Windows writes at most `size` bytes.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN,
            &name,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    result.ok().ok()?;
    let length = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buffer[..length]))
}

/// Starts `command` at login under `name`, or removes that entry when `command` is `None`.
///
/// # Errors
/// When Windows refuses to change the registry value.
pub fn set_autostart(name: &str, command: Option<&str>) -> windows::core::Result<()> {
    let name = HSTRING::from(name);
    let Some(command) = command else {
        // SAFETY: both strings are valid and null-terminated.
        let result = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN, &name) };
        return if result == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            result.ok()
        };
    };
    let data: Vec<u16> = command.encode_utf16().chain([0]).collect();
    // SAFETY: `data` is a null-terminated UTF-16 string of the given byte size.
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            RUN,
            &name,
            REG_SZ.0,
            Some(data.as_ptr().cast()),
            (data.len() * 2) as u32,
        )
    }
    .ok()
}
