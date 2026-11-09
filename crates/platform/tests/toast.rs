//! Windows itself confirms the toast: it must appear in the Action Center history for our ID.
//! Ignored by default because it shows a real (silent) toast; run with `cargo test -- --ignored`.

use std::time::{Duration, Instant};

use platform::toast;
use windows::UI::Notifications::ToastNotificationManager;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteTreeW};
use windows::core::HSTRING;

const APP_ID: &str = "StretchCore.Test";

#[test]
#[ignore = "shows a real toast"]
fn toast_reaches_windows_notification_history() {
    toast::register(APP_ID, "Stretch test").unwrap();
    let history = ToastNotificationManager::History().unwrap();
    let id = HSTRING::from(APP_ID);
    let _ = history.ClearWithId(&id);

    let end = Instant::now() + Duration::from_secs(15);
    let (mut count, mut sent, mut next_send) = (0, 0, Instant::now());
    while count == 0 && Instant::now() < end {
        // Resend every 3 s: the first toast after registration can be dropped (ADR 0001, R2).
        if Instant::now() >= next_send {
            toast::show(APP_ID, "Stretch test", "Automated check", true);
            sent += 1;
            next_send += Duration::from_secs(3);
        }
        std::thread::sleep(Duration::from_millis(200));
        count = history
            .GetHistoryWithId(&id)
            .map(|list| list.Size().unwrap())
            .unwrap_or(0);
    }

    let _ = history.ClearWithId(&id);
    let key = HSTRING::from(format!(r"Software\Classes\AppUserModelId\{APP_ID}"));
    // SAFETY: `key` is a valid wide string under HKCU that only this test created.
    let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &key) };
    assert!(count > 0, "no toast in history after {sent} sends");
    eprintln!("toast confirmed by Windows after {sent} send(s)");
}
