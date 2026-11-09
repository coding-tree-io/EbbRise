use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::HSTRING;

/// Returns `false` when another process already holds the named mutex `name`.
///
/// Use a `Local\` name for one instance per session. The mutex handle stays open for the
/// process lifetime. If the mutex cannot be created, this process counts as the first.
///
/// A second instance passes its command on with [`crate::send_command`].
pub fn is_first_instance(name: &str) -> bool {
    // SAFETY: `name` is converted to a valid null-terminated wide string for the call.
    match unsafe { CreateMutexW(None, true, &HSTRING::from(name)) } {
        // SAFETY: no arguments; reads this thread's last error right after CreateMutexW.
        Ok(_handle) => (unsafe { GetLastError() }) != ERROR_ALREADY_EXISTS,
        Err(_) => true,
    }
}

/// Lets a windowed (release) build print to the console it was started from, if any.
pub fn attach_parent_console() {
    // SAFETY: no pointers; fails harmlessly when there is no parent console.
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}
