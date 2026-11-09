//! Probe: prints each Windows signal the backend depends on, all on Slint's event loop.
//! Hotkeys: Ctrl+Alt+Shift+T shows a toast, Ctrl+Alt+Shift+Q quits. Output also goes to probe.log.

use std::ffi::c_void;
use std::io::Write;
use std::sync::atomic::{AtomicIsize, Ordering};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};
use windows::core::w;
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::{RegisterSuspendResumeNotification, POWERBROADCAST_SETTING};
use windows::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION};
use windows::Win32::System::SystemInformation::{GetLocalTime, GetTickCount};
use windows::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::Shell::SHQueryUserNotificationState;
use windows::Win32::UI::WindowsAndMessaging::*;

const APP_ID: &str = "StretchCore.Probe";
const TIMER_ID: usize = 1;
const WM_FOCUS_ASSIST: u32 = WM_APP + 1;
// WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED: undocumented, the same name Stretchly's windows-focus-assist reads.
const WNF_FOCUS_ASSIST: u64 = 0x0D83_063E_A3BF_1C75;

static WINDOW: AtomicIsize = AtomicIsize::new(0);

macro_rules! log {
    ($($arg:tt)*) => {{
        #[allow(unused_unsafe)]
        let t = unsafe { GetLocalTime() };
        let line = format!("{:02}:{:02}:{:02}.{:03}  {}", t.wHour, t.wMinute, t.wSecond, t.wMilliseconds, format!($($arg)*));
        println!("{line}");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("probe.log") {
            let _ = writeln!(f, "{line}");
        }
    }};
}

type WnfCallback = unsafe extern "system" fn(u64, u32, *const c_void, *const c_void, *const c_void, u32) -> i32;

#[link(name = "ntdll")]
extern "system" {
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

fn main() {
    log!("probe start");
    let window = create_signal_window();
    WINDOW.store(window.0 as isize, Ordering::SeqCst);

    unsafe {
        report(
            "session notifications",
            WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION).map(|_| ()),
        );
        report(
            "suspend/resume notifications",
            RegisterSuspendResumeNotification(HANDLE(window.0), DEVICE_NOTIFY_WINDOW_HANDLE).map(|_| ()),
        );
        let timer = SetCoalescableTimer(Some(window), TIMER_ID, 10_000, None, 1_000);
        log!("coalescable timer 10 s, tolerance 1 s: {}", if timer != 0 { "ok" } else { "FAILED" });
    }
    subscribe_focus_assist();
    register_toast_app_id();

    let _tray = TrayIconBuilder::new()
        .with_tooltip("Stretch probe")
        .with_icon(Icon::from_rgba([40u8, 160, 120, 255].repeat(16 * 16), 16, 16).unwrap())
        .build()
        .expect("tray icon");
    TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
        if let TrayIconEvent::Click { button, button_state, .. } = event {
            log!("tray click: {button:?} {button_state:?}");
        }
    }));

    let hotkeys = GlobalHotKeyManager::new().expect("hotkey manager");
    let mods = Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT);
    let toast_key = HotKey::new(mods, Code::KeyT);
    let quit_key = HotKey::new(mods, Code::KeyQ);
    report("hotkey Ctrl+Alt+Shift+T", hotkeys.register(toast_key));
    report("hotkey Ctrl+Alt+Shift+Q", hotkeys.register(quit_key));
    let (toast_id, quit_id) = (toast_key.id(), quit_key.id());
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state != HotKeyState::Pressed {
            return;
        }
        if event.id == toast_id {
            log!("hotkey: toast");
            let shown = tauri_winrt_notification::Toast::new(APP_ID)
                .title("Stretch probe")
                .text1("Toast from an unpackaged exe with a registry AppUserModelID.")
                .show();
            log!("toast show (own id): {shown:?}");
            let shown = tauri_winrt_notification::Toast::new(tauri_winrt_notification::Toast::POWERSHELL_APP_ID)
                .title("Stretch probe (PowerShell id)")
                .text1("Same toast, sent with PowerShell's registered id.")
                .show();
            log!("toast show (PowerShell id): {shown:?}");
        } else if event.id == quit_id {
            log!("hotkey: quit");
            let _ = slint::quit_event_loop();
        }
    }));

    log!("Slint event loop running with no window open");
    slint::run_event_loop_until_quit().expect("slint event loop");
    log!("probe end");
}

fn report<E: std::fmt::Debug>(what: &str, result: Result<(), E>) {
    match result {
        Ok(()) => log!("{what}: ok"),
        Err(e) => log!("{what}: FAILED {e:?}"),
    }
}

/// Hidden top-level window: it receives WM_POWERBROADCAST, which message-only windows do not.
fn create_signal_window() -> HWND {
    unsafe {
        let instance = GetModuleHandleW(None).expect("module handle");
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: w!("StretchProbeSignals"),
            ..Default::default()
        };
        RegisterClassW(&class);
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("StretchProbeSignals"),
            w!("Stretch probe signals"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .expect("signal window")
    }
}

unsafe extern "system" fn window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_TIMER if wparam.0 == TIMER_ID => {
            log!("timer tick  {}", clocks());
            log!("  idle {} ms, notification state {:?}", idle_ms(), SHQueryUserNotificationState().map(|s| s.0));
        }
        WM_POWERBROADCAST => {
            let name = match wparam.0 as u32 {
                PBT_APMSUSPEND => "suspend",
                PBT_APMRESUMESUSPEND => "resume (user)",
                PBT_APMRESUMEAUTOMATIC => "resume (automatic)",
                PBT_APMPOWERSTATUSCHANGE => "power status change",
                PBT_POWERSETTINGCHANGE => {
                    let setting = &*(lparam.0 as *const POWERBROADCAST_SETTING);
                    log!("power setting change {:?}", setting.PowerSetting);
                    return LRESULT(1);
                }
                _ => "other",
            };
            log!("power: {name} (0x{:x})  {}", wparam.0, clocks());
            return LRESULT(1);
        }
        WM_WTSSESSION_CHANGE => {
            let name = match wparam.0 as u32 {
                WTS_SESSION_LOCK => "lock",
                WTS_SESSION_UNLOCK => "unlock",
                WTS_CONSOLE_CONNECT => "console connect",
                WTS_CONSOLE_DISCONNECT => "console disconnect",
                _ => "other",
            };
            log!("session: {name} ({})  {}", wparam.0, clocks());
        }
        WM_FOCUS_ASSIST => log!("focus assist changed: {}", wparam.0 as i32),
        _ => {}
    }
    DefWindowProcW(window, message, wparam, lparam)
}

/// Tick count and unbiased interrupt time both in seconds; the gap between them grows by the time spent asleep.
fn clocks() -> String {
    let mut unbiased = 0u64;
    unsafe {
        let _ = QueryUnbiasedInterruptTime(&mut unbiased);
        format!("[tick {:.1} s, awake {:.1} s]", GetTickCount() as f64 / 1000.0, unbiased as f64 / 1e7)
    }
}

fn idle_ms() -> u32 {
    let mut info = LASTINPUTINFO { cbSize: size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe {
        if GetLastInputInfo(&mut info).as_bool() {
            GetTickCount().wrapping_sub(info.dwTime)
        } else {
            0
        }
    }
}

fn subscribe_focus_assist() {
    let mut stamp = 0u32;
    let mut value = 0i32;
    let mut size = size_of::<i32>() as u32;
    let status = unsafe {
        NtQueryWnfStateData(
            &WNF_FOCUS_ASSIST,
            std::ptr::null(),
            std::ptr::null(),
            &mut stamp,
            &mut value as *mut i32 as *mut c_void,
            &mut size,
        )
    };
    log!("focus assist initial: status 0x{status:x}, value {value}, size {size}, stamp {stamp}");

    let mut subscription = std::ptr::null_mut();
    let status = unsafe {
        RtlSubscribeWnfStateChangeNotification(
            &mut subscription,
            WNF_FOCUS_ASSIST,
            stamp,
            on_focus_assist,
            std::ptr::null(),
            std::ptr::null(),
            0,
            0,
        )
    };
    log!("focus assist subscription: status 0x{status:x}");
}

/// Runs on a thread-pool thread, so it only posts the value to the window's thread.
unsafe extern "system" fn on_focus_assist(
    _state: u64,
    _stamp: u32,
    _type_id: *const c_void,
    _context: *const c_void,
    buffer: *const c_void,
    length: u32,
) -> i32 {
    let value = if length >= 4 && !buffer.is_null() { *(buffer as *const i32) } else { -1 };
    let window = HWND(WINDOW.load(Ordering::SeqCst) as *mut c_void);
    let _ = PostMessageW(Some(window), WM_FOCUS_ASSIST, WPARAM(value as usize), LPARAM(0));
    0
}

/// Unpackaged toasts need an AppUserModelID; registering it under HKCU is the documented no-installer route (risk R2).
fn register_toast_app_id() {
    let key = format!(r"HKCU\Software\Classes\AppUserModelId\{APP_ID}");
    let status = std::process::Command::new("reg")
        .args(["add", &key, "/v", "DisplayName", "/d", "Stretch probe", "/f"])
        .output()
        .map(|o| o.status.success());
    log!("toast app id registry entry: {status:?}");
}
