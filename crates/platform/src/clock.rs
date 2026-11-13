use std::sync::Once;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows::Win32::Foundation::HWND;
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::Input::{RAWINPUTDEVICE, RIDEV_INPUTSINK, RegisterRawInputDevices};

const HUNDRED_NS_PER_MS: u64 = 10_000;
static RAW_INPUT_READY: AtomicBool = AtomicBool::new(false);
static LAST_RAW_INPUT: AtomicU32 = AtomicU32::new(0);
static REGISTRATION_WARNING: Once = Once::new();

pub(crate) fn register_input(window: HWND) {
    let devices = [2, 6].map(|usage| RAWINPUTDEVICE {
        usUsagePage: 1,
        usUsage: usage,
        dwFlags: RIDEV_INPUTSINK,
        hwndTarget: window,
    });
    // SAFETY: both device entries and their target window are valid.
    match unsafe { RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32) } {
        Ok(()) => {
            record_input();
            RAW_INPUT_READY.store(true, Ordering::Release);
        }
        Err(error) => {
            RAW_INPUT_READY.store(false, Ordering::Release);
            REGISTRATION_WARNING.call_once(|| {
                tracing::warn!(%error, "raw input registration failed; using Windows idle time");
            });
        }
    }
}

pub(crate) fn record_input() {
    // SAFETY: no arguments.
    LAST_RAW_INPUT.store(unsafe { GetTickCount() }, Ordering::Relaxed);
}

/// Milliseconds the PC has been awake since boot.
///
/// Sleep time is excluded, so a deadline keeps its remaining time across sleep, like
/// upstream's `correctScheduler` (ADR 0001, R3).
pub fn awake_ms() -> i64 {
    let mut hundred_ns = 0u64;
    // SAFETY: `hundred_ns` is a valid, writable u64; the call cannot fail with it.
    let _ = unsafe { QueryUnbiasedInterruptTime(&mut hundred_ns) };
    (hundred_ns / HUNDRED_NS_PER_MS) as i64
}

/// Milliseconds since the last keyboard or mouse input in this session; 0 if unknown.
pub fn idle_ms() -> i64 {
    if RAW_INPUT_READY.load(Ordering::Acquire) {
        let last_input = LAST_RAW_INPUT.load(Ordering::Relaxed);
        // SAFETY: no arguments. Both ticks wrap at the same 32-bit boundary.
        return i64::from(unsafe { GetTickCount() }.wrapping_sub(last_input));
    }
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `info` is a valid LASTINPUTINFO with `cbSize` set, as the API requires.
    if unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        // Both are 32-bit tick counts; wrapping_sub stays right across the 49-day wrap.
        // SAFETY: no arguments.
        i64::from(unsafe { GetTickCount() }.wrapping_sub(info.dwTime))
    } else {
        0
    }
}
