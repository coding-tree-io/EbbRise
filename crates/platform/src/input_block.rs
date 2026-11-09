//! Keeps keyboard and mouse input away from other apps during a break
//! (`blockInputDuringBreaks`): low-level hooks pass input that goes to this app's windows and
//! drop the rest. Windows never lets a hook block Ctrl+Alt+Del, so the user can always get out.

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetWindowThreadProcessId, HC_ACTION, HHOOK,
    MSLLHOOKSTRUCT, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WM_MOUSEMOVE, WindowFromPoint,
};

/// The block; dropping it ends it.
pub struct InputBlock {
    keyboard: HHOOK,
    mouse: HHOOK,
}

impl InputBlock {
    /// Starts the block. Call it on a thread that pumps messages, which the hooks need.
    ///
    /// # Errors
    /// When Windows refuses a hook.
    pub fn start() -> windows::core::Result<Self> {
        // SAFETY: a null name asks for this exe's own module handle.
        let module = unsafe { GetModuleHandleW(None) }?;
        // SAFETY: the callbacks have the HOOKPROC signature and live for the whole program;
        // thread id 0 with a module handle is how low-level hooks are installed.
        let keyboard =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), Some(module.into()), 0) }?;
        // SAFETY: as above.
        let mouse =
            match unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), Some(module.into()), 0) } {
                Ok(mouse) => mouse,
                Err(error) => {
                    // SAFETY: `keyboard` was installed above and is not used after this.
                    let _ = unsafe { UnhookWindowsHookEx(keyboard) };
                    return Err(error);
                }
            };
        tracing::info!("input blocked");
        Ok(Self { keyboard, mouse })
    }
}

impl Drop for InputBlock {
    fn drop(&mut self) {
        // SAFETY: both hooks were installed by `start` and are removed once, here.
        unsafe {
            let _ = UnhookWindowsHookEx(self.keyboard);
            let _ = UnhookWindowsHookEx(self.mouse);
        }
        tracing::info!("input unblocked");
    }
}

fn ours(window: HWND) -> bool {
    let mut pid = 0;
    // SAFETY: `pid` is a valid, writable u32; a null or stale window gives 0.
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    // SAFETY: no arguments.
    pid != 0 && pid == unsafe { GetCurrentProcessId() }
}

/// Passes a key only to this app's window in the foreground.
extern "system" fn keyboard(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: no arguments.
    if code == HC_ACTION as i32 && !ours(unsafe { GetForegroundWindow() }) {
        return LRESULT(1);
    }
    // SAFETY: the arguments are the ones Windows passed in.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Passes moves everywhere, so the pointer can reach the break window, and clicks and wheel
/// turns only over this app's windows.
extern "system" fn mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wparam.0 as u32 != WM_MOUSEMOVE {
        // SAFETY: for WH_MOUSE_LL with HC_ACTION, `lparam` points to an MSLLHOOKSTRUCT.
        let point = unsafe { (*(lparam.0 as *const MSLLHOOKSTRUCT)).pt };
        // SAFETY: plain call with a point.
        if !ours(unsafe { WindowFromPoint(point) }) {
            return LRESULT(1);
        }
    }
    // SAFETY: the arguments are the ones Windows passed in.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
