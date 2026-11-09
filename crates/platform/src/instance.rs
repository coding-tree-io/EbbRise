use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::HSTRING;

/// Returns `false` when another process already holds the named mutex `name`.
///
/// Use a `Local\` name for one instance per session. The mutex handle stays open for the
/// process lifetime. If the mutex cannot be created, this process counts as the first.
///
/// shortcut: a second instance just exits; forward its CLI command over a named pipe when the CLI lands.
pub fn is_first_instance(name: &str) -> bool {
    // SAFETY: `name` is converted to a valid null-terminated wide string for the call.
    match unsafe { CreateMutexW(None, true, &HSTRING::from(name)) } {
        // SAFETY: no arguments; reads this thread's last error right after CreateMutexW.
        Ok(_handle) => (unsafe { GetLastError() }) != ERROR_ALREADY_EXISTS,
        Err(_) => true,
    }
}
