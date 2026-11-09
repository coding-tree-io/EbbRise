use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

const HUNDRED_NS_PER_MS: u64 = 10_000;

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
