//! One hidden window on the UI thread receives every OS signal the backend uses, and one
//! coalescable timer. Any message loop on this thread (Slint's included) pumps it.

use std::cell::RefCell;

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::RegisterSuspendResumeNotification;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

use crate::focus_assist::{self, WM_FOCUS_ASSIST};

/// An OS event delivered to the handler given to [`SignalWindow::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// The timer armed with [`SignalWindow::arm`] fired.
    Timer,
    /// The PC is going to sleep.
    Suspend,
    /// The PC woke up.
    Resume,
    /// The session was locked.
    Lock,
    /// The session was unlocked.
    Unlock,
    /// New Focus Assist value: 0 off, 1 priority only, 2 alarms only.
    FocusAssist(i32),
}

/// The hidden window that receives power, session and Focus Assist events and the timer.
pub struct SignalWindow {
    window: HWND,
    /// Focus Assist value at start, or `None` when it cannot be read.
    pub focus_assist: Option<i32>,
}

const TIMER_ID: usize = 1;

type Handler = Box<dyn FnMut(Signal)>;

thread_local! {
    static HANDLER: RefCell<Option<Handler>> = RefCell::new(None);
}

impl SignalWindow {
    /// Creates the window on the calling thread; `on_signal` runs on that thread.
    ///
    /// Create one per thread: a second call replaces the thread's handler.
    ///
    /// # Errors
    /// When Windows cannot create the window or register it for session or power events.
    pub fn new(on_signal: impl FnMut(Signal) + 'static) -> windows::core::Result<Self> {
        HANDLER.with(|handler| *handler.borrow_mut() = Some(Box::new(on_signal)));
        // SAFETY: the class and window names are static wide strings, and `window_proc`
        // matches WNDPROC. A failed RegisterClassW (class already registered) is harmless.
        let window = unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                lpszClassName: w!("StretchCoreSignals"),
                ..Default::default()
            };
            RegisterClassW(&class);
            // A hidden top-level window, not message-only: only top-level windows get WM_POWERBROADCAST.
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("StretchCoreSignals"),
                w!("stretch-core signals"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance.into()),
                None,
            )?
        };
        // SAFETY: `window` is the valid window created above and lives for the process.
        unsafe {
            WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION)?;
            RegisterSuspendResumeNotification(HANDLE(window.0), DEVICE_NOTIFY_WINDOW_HANDLE)?;
        }
        let focus_assist = focus_assist::subscribe(window);
        Ok(Self {
            window,
            focus_assist,
        })
    }

    /// Fires [`Signal::Timer`] once after `delay_ms`, replacing any armed timer.
    ///
    /// The delay is clamped to 10 ms–24.8 days. `tolerance_ms` lets Windows batch the wake-up.
    pub fn arm(&self, delay_ms: i64, tolerance_ms: u32) {
        let delay = delay_ms.clamp(USER_TIMER_MINIMUM as i64, USER_TIMER_MAXIMUM as i64) as u32;
        // SAFETY: `self.window` is a valid window owned by this thread.
        unsafe { SetCoalescableTimer(Some(self.window), TIMER_ID, delay, None, tolerance_ms) };
    }

    /// The hidden window's handle, for sending it messages.
    pub fn window(&self) -> HWND {
        self.window
    }

    /// Cancels the armed timer, if any.
    pub fn disarm(&self) {
        // SAFETY: `self.window` is a valid window owned by this thread.
        let _ = unsafe { KillTimer(Some(self.window), TIMER_ID) };
    }
}

/// Runs the Win32 message loop on this thread until [`quit`].
///
/// shortcut: own loop until the Slint UI exists; then Slint's loop pumps the same window.
pub fn run_message_loop() {
    let mut message = MSG::default();
    // SAFETY: `message` is a valid MSG that outlives each call.
    unsafe {
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Ends [`run_message_loop`] on this thread.
pub fn quit() {
    // SAFETY: no pointers; posts WM_QUIT to this thread.
    unsafe { PostQuitMessage(0) };
}

fn dispatch(signal: Signal) {
    HANDLER.with(|handler| {
        if let Some(handler) = handler.borrow_mut().as_mut() {
            handler(signal);
        }
    });
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let signal = match message {
        WM_TIMER if wparam.0 == TIMER_ID => {
            // One-shot: the handler re-arms for the next deadline.
            // SAFETY: `window` is the window this procedure was called for.
            let _ = unsafe { KillTimer(Some(window), TIMER_ID) };
            Some(Signal::Timer)
        }
        WM_POWERBROADCAST => match wparam.0 as u32 {
            PBT_APMSUSPEND => Some(Signal::Suspend),
            PBT_APMRESUMEAUTOMATIC => Some(Signal::Resume),
            _ => None,
        },
        WM_WTSSESSION_CHANGE => match wparam.0 as u32 {
            WTS_SESSION_LOCK => Some(Signal::Lock),
            WTS_SESSION_UNLOCK => Some(Signal::Unlock),
            _ => None,
        },
        WM_FOCUS_ASSIST => Some(Signal::FocusAssist(wparam.0 as u32 as i32)),
        _ => None,
    };
    if let Some(signal) = signal {
        dispatch(signal);
        // Windows expects TRUE back from power broadcasts.
        return LRESULT((message == WM_POWERBROADCAST).into());
    }
    // SAFETY: forwards the unchanged arguments Windows passed in.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}
