//! Brings a window of this app to the front with keyboard focus.

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Threading::AttachThreadInput;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumThreadWindows, GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow,
    GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, SW_HIDE, SW_SHOW,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
};
use windows::core::BOOL;

/// Gives the window titled `title` on this thread the foreground and keyboard focus; returns
/// whether such a window was found.
///
/// Windows refuses the foreground to a background app, so this joins the input of the
/// current foreground window's thread for the call, the usual way around that rule.
pub fn bring_to_front(title: &str) -> bool {
    let Some(&window) = own_windows(title).first() else {
        return false;
    };
    // SAFETY: `window` is a live window of this thread; the thread ids come from Windows.
    unsafe {
        let own_thread = GetCurrentThreadId();
        let foreground_thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let joined = foreground_thread != 0
            && foreground_thread != own_thread
            && AttachThreadInput(own_thread, foreground_thread, true).as_bool();
        let _ = BringWindowToTop(window);
        let _ = SetForegroundWindow(window);
        let _ = SetFocus(Some(window));
        if joined {
            let _ = AttachThreadInput(own_thread, foreground_thread, false);
        }
    }
    true
}

/// Takes the minimize and maximize styles and the taskbar button from every window titled
/// `title` on this thread (one break window per display), as Stretchly's `skipTaskbar` break
/// windows; returns whether one was found. Win+Down, Win+M and a taskbar click then have
/// nothing to minimize.
pub fn lock_window(title: &str) -> bool {
    let windows = own_windows(title);
    for &window in &windows {
        lock(window);
    }
    !windows.is_empty()
}

fn lock(window: HWND) {
    // SAFETY: `window` is a live window of this thread; the styles are plain bit flags.
    unsafe {
        let style = GetWindowLongPtrW(window, GWL_STYLE) as u32;
        SetWindowLongPtrW(
            window,
            GWL_STYLE,
            (style & !(WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0)) as isize,
        );
        let ex_style = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(
            window,
            GWL_EXSTYLE,
            ((ex_style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0) as isize,
        );
        // The taskbar drops the button only when the window is shown again.
        let _ = ShowWindow(window, SW_HIDE);
        let _ = ShowWindow(window, SW_SHOW);
    }
}

/// The top-level windows of this thread titled `title`, in z-order; another app's window
/// never matches.
fn own_windows(title: &str) -> Vec<HWND> {
    struct Search {
        title: Vec<u16>,
        found: Vec<HWND>,
    }
    unsafe extern "system" fn visit(window: HWND, search: LPARAM) -> BOOL {
        // SAFETY: `search` is the `Search` that `own_windows` passes and keeps alive.
        let search = unsafe { &mut *(search.0 as *mut Search) };
        let mut text = [0u16; 256];
        // SAFETY: `text` is a writable buffer of the length the call is given.
        let length = unsafe { GetWindowTextW(window, &mut text) } as usize;
        if text[..length] == search.title[..] {
            search.found.push(window);
        }
        true.into()
    }
    let mut search = Search {
        title: title.encode_utf16().collect(),
        found: Vec::new(),
    };
    // SAFETY: `visit` reads `search` only during this call, while it is alive.
    let _ = unsafe {
        EnumThreadWindows(
            GetCurrentThreadId(),
            Some(visit),
            LPARAM((&raw mut search) as isize),
        )
    };
    search.found
}
