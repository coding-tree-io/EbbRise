//! One hidden window on the UI thread receives every OS signal the backend uses, and one
//! coalescable timer. Any message loop on this thread (Slint's included) pumps it.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::RegisterSuspendResumeNotification;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, w};

use crate::focus_assist::{self, WM_FOCUS_ASSIST};

/// An OS event delivered to the handler given to [`SignalWindow::new`].
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Windows is ending the session.
    SessionEnd,
    /// New Focus Assist value: 0 off, 1 priority only, 2 alarms only.
    FocusAssist(i32),
    /// Command-line arguments that a second instance sent with [`send_command`].
    Command(Vec<String>),
}

/// The hidden window that receives power, session and Focus Assist events and the timer.
pub struct SignalWindow {
    window: HWND,
    /// Focus Assist value at start, or `None` when it cannot be read.
    pub focus_assist: Option<i32>,
}

const TIMER_ID: usize = 1;
const CLASS_NAME: PCWSTR = w!("EbbRiseSignals");
/// Marks our WM_COPYDATA messages, so data from other programs is ignored.
const COMMAND_TAG: usize = 0x5354_4331;
/// Arguments longer than this are not a real command line.
const MAX_COMMAND_BYTES: u32 = 64 * 1024;
const ARGUMENT_SEPARATOR: char = '\0';

type Handler = Box<dyn FnMut(Signal)>;

thread_local! {
    static HANDLER: RefCell<Option<Handler>> = RefCell::new(None);
    /// Signals that arrived while the handler ran. A Windows call inside the handler can
    /// deliver a sent message to this thread; that signal waits here instead of re-entering.
    static PENDING: RefCell<VecDeque<Signal>> = const { RefCell::new(VecDeque::new()) };
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
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };
            RegisterClassW(&class);
            // A hidden top-level window, not message-only: only top-level windows get WM_POWERBROADCAST.
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                CLASS_NAME,
                w!("EbbRise signals"),
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
        crate::clock::register_input(window);
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

/// The hidden window of the running app in this session, if one runs.
///
/// Waits up to 5 s, because the app takes the single-instance mutex before it creates the
/// window.
pub fn running_app_window() -> Option<HWND> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        // SAFETY: `CLASS_NAME` is a static wide string.
        let window = unsafe { FindWindowW(CLASS_NAME, None) }.ok();
        if window.is_some() || Instant::now() >= deadline {
            return window;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Sends command-line arguments to `window`, which delivers them as [`Signal::Command`].
///
/// Waits up to 5 s for the app to take them. Returns `false` when it does not.
pub fn send_command(window: HWND, args: &[String]) -> bool {
    let data = args.join(&ARGUMENT_SEPARATOR.to_string());
    let Ok(length) = u32::try_from(data.len()) else {
        return false;
    };
    let copy = COPYDATASTRUCT {
        dwData: COMMAND_TAG,
        cbData: length,
        lpData: data.as_ptr() as *mut _,
    };
    let mut result = 0;
    // SAFETY: `copy` and `data` outlive the call; Windows copies the bytes to the receiver.
    let sent = unsafe {
        SendMessageTimeoutW(
            window,
            WM_COPYDATA,
            WPARAM(0),
            LPARAM(&raw const copy as isize),
            SMTO_ABORTIFHUNG,
            5_000,
            Some(&mut result),
        )
    };
    sent.0 != 0 && result != 0
}

/// Hands `signal` to the handler, after any signals that are still waiting.
fn dispatch(signal: Signal) {
    PENDING.with(|pending| pending.borrow_mut().push_back(signal));
    HANDLER.with(|handler| {
        // Busy: the handler call that is running takes this signal when it is done.
        let Ok(mut handler) = handler.try_borrow_mut() else {
            return;
        };
        let Some(handler) = handler.as_mut() else {
            return;
        };
        while let Some(signal) = PENDING.with(|pending| pending.borrow_mut().pop_front()) {
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
        WM_INPUT => {
            crate::clock::record_input();
            None
        }
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
        WM_QUERYENDSESSION => return LRESULT(1),
        WM_ENDSESSION if wparam.0 != 0 => Some(Signal::SessionEnd),
        // SAFETY: Windows passes a COPYDATASTRUCT that is valid during WM_COPYDATA.
        WM_COPYDATA => unsafe { command_arguments(lparam) }.map(Signal::Command),
        _ => None,
    };
    if let Some(signal) = signal {
        dispatch(signal);
        // Windows expects TRUE back from power broadcasts, and the sender of a command.
        return LRESULT(matches!(message, WM_POWERBROADCAST | WM_COPYDATA).into());
    }
    // SAFETY: forwards the unchanged arguments Windows passed in.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// The arguments in a WM_COPYDATA message from [`send_command`]; `None` for other data.
///
/// # Safety
/// `lparam` must be the COPYDATASTRUCT pointer of a WM_COPYDATA message being handled.
unsafe fn command_arguments(lparam: LPARAM) -> Option<Vec<String>> {
    // SAFETY: the caller passes the COPYDATASTRUCT that Windows keeps valid during the call.
    let copy = unsafe { &*(lparam.0 as *const COPYDATASTRUCT) };
    if copy.dwData != COMMAND_TAG || copy.cbData > MAX_COMMAND_BYTES {
        return None;
    }
    if copy.cbData == 0 {
        return Some(Vec::new());
    }
    // SAFETY: Windows guarantees `cbData` readable bytes at `lpData` during the call.
    let bytes =
        unsafe { std::slice::from_raw_parts(copy.lpData as *const u8, copy.cbData as usize) };
    let text = String::from_utf8_lossy(bytes);
    Some(text.split(ARGUMENT_SEPARATOR).map(str::to_owned).collect())
}
