//! Real Windows, no person: messages go to the real hidden window, the real timer fires,
//! and injected input resets the real idle clock.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use platform::{Signal, SignalWindow, awake_ms, idle_ms};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::*;

type Received = Rc<RefCell<Vec<(Signal, Instant)>>>;

fn signal_window() -> (SignalWindow, Received) {
    let received = Received::default();
    let sink = received.clone();
    let signals =
        SignalWindow::new(move |signal| sink.borrow_mut().push((signal, Instant::now()))).unwrap();
    (signals, received)
}

fn pump(time: Duration) {
    let end = Instant::now() + time;
    let mut message = MSG::default();
    while Instant::now() < end {
        // SAFETY: `message` is a valid MSG that outlives each call.
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            // SAFETY: `message` was just filled in by PeekMessageW.
            unsafe { DispatchMessageW(&message) };
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn os_messages_become_signals_and_other_values_are_ignored() {
    let (signals, received) = signal_window();
    let window = signals.window();
    assert!(
        signals.focus_assist.is_some(),
        "Focus Assist state readable through WNF"
    );
    // SAFETY: `window` is a live window on this thread; the parameters carry no pointers.
    let send = |message, value: u32| unsafe {
        SendMessageW(
            window,
            message,
            Some(WPARAM(value as usize)),
            Some(LPARAM(0)),
        )
    };
    // Windows needs TRUE back from power broadcasts.
    assert_eq!(send(WM_POWERBROADCAST, PBT_APMSUSPEND).0, 1);
    send(WM_POWERBROADCAST, PBT_APMRESUMEAUTOMATIC);
    send(WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK);
    send(WM_WTSSESSION_CHANGE, WTS_SESSION_UNLOCK);
    send(WM_POWERBROADCAST, PBT_POWERSETTINGCHANGE);
    // Focus Assist changes arrive posted from a thread-pool thread.
    // SAFETY: as above.
    unsafe { PostMessageW(Some(window), WM_APP + 1, WPARAM(2), LPARAM(0)) }.unwrap();
    pump(Duration::from_millis(20));
    let signals_only: Vec<_> = received
        .take()
        .into_iter()
        .map(|(signal, _)| signal)
        .collect();
    assert_eq!(
        signals_only,
        [
            Signal::Suspend,
            Signal::Resume,
            Signal::Lock,
            Signal::Unlock,
            Signal::FocusAssist(2)
        ]
    );
}

#[test]
fn armed_timer_fires_once_near_its_delay() {
    let (signals, received) = signal_window();
    let armed = Instant::now();
    signals.arm(50, 0);
    // Long enough for a periodic timer to fire a second time.
    pump(Duration::from_millis(160));
    let fired = received.take();
    assert_eq!(fired.len(), 1, "one-shot timer");
    let late = fired[0].1 - armed;
    assert!(
        (Duration::from_millis(45)..Duration::from_millis(120)).contains(&late),
        "{late:?}"
    );
}

#[test]
fn disarmed_timer_never_fires() {
    let (signals, received) = signal_window();
    signals.arm(30, 0);
    signals.disarm();
    pump(Duration::from_millis(80));
    assert!(received.borrow().is_empty());
}

#[test]
fn clocks_follow_real_time_and_real_input() {
    let start = awake_ms();
    std::thread::sleep(Duration::from_millis(100));
    let passed = awake_ms() - start;
    assert!((90..200).contains(&passed), "{passed} ms");

    // A zero-distance mouse move counts as input but moves nothing.
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: MOUSEEVENTF_MOVE,
                ..Default::default()
            },
        },
    };
    // SAFETY: one valid INPUT with its correct size.
    assert_eq!(unsafe { SendInput(&[input], size_of::<INPUT>() as i32) }, 1);
    assert!(idle_ms() < 50, "idle reset by input: {} ms", idle_ms());
}
