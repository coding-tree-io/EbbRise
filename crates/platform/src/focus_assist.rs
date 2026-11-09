//! Focus Assist / Do Not Disturb through the undocumented WNF state that Stretchly's
//! `windows-focus-assist` addon also reads. Verified on Windows 11 build 26300 (ADR 0001, R1):
//! 0 = off, 1 = priority only, 2 = alarms only / DND on.

use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Posted to the subscribed window; `wparam` holds the new value.
pub(crate) const WM_FOCUS_ASSIST: u32 = WM_APP + 1;

const WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED: u64 = 0x0D83_063E_A3BF_1C75;

type WnfCallback =
    unsafe extern "system" fn(u64, u32, *const c_void, *const c_void, *const c_void, u32) -> i32;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlSubscribeWnfStateChangeNotification(
        subscription: *mut *mut c_void,
        state_name: u64,
        change_stamp: u32,
        callback: WnfCallback,
        context: *const c_void,
        type_id: *const c_void,
        serialization_group: u32,
        unknown: u32,
    ) -> i32;
    fn NtQueryWnfStateData(
        state_name: *const u64,
        type_id: *const c_void,
        explicit_scope: *const c_void,
        change_stamp: *mut u32,
        buffer: *mut c_void,
        buffer_size: *mut u32,
    ) -> i32;
}

/// Reads the current value and subscribes to changes, which are posted to `window` as
/// [`WM_FOCUS_ASSIST`] from a thread-pool thread. Returns `None` when the state is not
/// available (Stretchly's "unsupported", -1), in which case DND is treated as off.
pub(crate) fn subscribe(window: HWND) -> Option<i32> {
    let mut stamp = 0u32;
    let mut value = 0i32;
    let mut size = size_of::<i32>() as u32;
    // SAFETY: every pointer refers to a live local of the size passed in `size`.
    let status = unsafe {
        NtQueryWnfStateData(
            &WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED,
            std::ptr::null(),
            std::ptr::null(),
            &mut stamp,
            (&raw mut value).cast(),
            &mut size,
        )
    };
    if status != 0 {
        return None;
    }
    let mut subscription = std::ptr::null_mut();
    // SAFETY: `on_wnf_change` matches the callback type. The subscription is never removed,
    // so it lives as long as the process, and posting to a destroyed window just fails.
    let status = unsafe {
        RtlSubscribeWnfStateChangeNotification(
            &mut subscription,
            WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED,
            stamp,
            on_wnf_change,
            window.0,
            std::ptr::null(),
            0,
            0,
        )
    };
    (status == 0).then_some(value)
}

unsafe extern "system" fn on_wnf_change(
    _state_name: u64,
    _change_stamp: u32,
    _type_id: *const c_void,
    window: *const c_void,
    buffer: *const c_void,
    length: u32,
) -> i32 {
    let value = if length as usize >= size_of::<i32>() && !buffer.is_null() {
        // SAFETY: WNF passes `length` readable bytes at `buffer`; it does not promise alignment.
        unsafe { buffer.cast::<i32>().read_unaligned() }
    } else {
        -1
    };
    // SAFETY: `window` is the handle given to `subscribe`; PostMessageW checks it.
    let _ = unsafe {
        PostMessageW(
            Some(HWND(window.cast_mut())),
            WM_FOCUS_ASSIST,
            WPARAM(value as u32 as usize),
            LPARAM(0),
        )
    };
    0
}
