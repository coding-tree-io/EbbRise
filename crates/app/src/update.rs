//! The update check, as upstream's `checkVersion`: shortly after start and then every 48
//! hours, read the latest release tag on GitHub. A newer one adds "Download latest version"
//! to the top of the tray menu and, when `notifyNewVersion` is on, shows a notification.
//! "Check for updates" in the tray menu and on the About page checks at once and always reports the result.

use std::time::Duration;

use slint::Timer;
use tracing::{info, warn};

use crate::logic::{MenuAction, MenuEntry};
use crate::{APP_ID, App, with_app};

const HOST: &str = "api.github.com";
const LATEST: &str = "/repos/coding-tree-io/EbbRise/releases/latest";
/// Where "Download latest version" goes.
pub(crate) const RELEASES: &str = "https://github.com/coding-tree-io/EbbRise/releases/latest";
const FIRST_CHECK: Duration = Duration::from_secs(10);
const EVERY: Duration = Duration::from_secs(48 * 3600);

/// Plans the first check; each check plans the next one.
pub(crate) fn start() {
    Timer::single_shot(FIRST_CHECK, check);
}

fn check() {
    Timer::single_shot(EVERY, check);
    with_app(|app| {
        if app.core.settings().check_new_version {
            // WinHTTP blocks, so the request runs on its own thread and reports back.
            std::thread::spawn(|| fetch_and_report(false));
        }
    });
}

/// Checks now, whatever `checkNewVersion` says, and reports the result in a notification.
pub(crate) fn check_now() {
    std::thread::spawn(|| fetch_and_report(true));
}

/// `manual`: the user asked, so "up to date" and a failed check are reported too.
fn fetch_and_report(manual: bool) {
    let tag = latest_tag();
    if tag.is_some() || manual {
        let _ = slint::invoke_from_event_loop(move || {
            with_app(move |app| app.on_latest_tag(tag.as_deref(), manual));
        });
    }
}

fn latest_tag() -> Option<String> {
    let agent = format!("{APP_ID}/{}", env!("CARGO_PKG_VERSION"));
    match platform::https_get(HOST, LATEST, &agent) {
        Ok((200, body)) => {
            let release: serde_json::Value = serde_json::from_slice(&body).ok()?;
            release["tag_name"].as_str().map(str::to_owned)
        }
        Ok((status, _)) => {
            info!(status, "no latest release");
            None
        }
        Err(error) => {
            warn!(%error, "update check failed");
            None
        }
    }
}

impl App {
    fn on_latest_tag(&mut self, tag: Option<&str>, manual: bool) {
        let current = env!("CARGO_PKG_VERSION");
        let Some(tag) = tag.filter(|tag| newer(current, tag)) else {
            let title = match tag {
                Some(_) => "EbbRise is up to date",
                None => "Could not check for updates",
            };
            if manual {
                self.notify(title, &format!("Version {current}"));
            }
            return;
        };
        let known = self.new_version.as_deref() == Some(tag);
        if known && !manual {
            return;
        }
        info!(tag, "new version");
        self.new_version = Some(tag.to_owned());
        if manual || self.core.settings().notify_new_version {
            self.notify(&self.core.words().text("process.newVersionAvailable"), tag);
        }
        self.refresh_tray();
    }

    fn notify(&self, title: &str, text: &str) {
        info!(title, "toast");
        platform::toast::show(
            APP_ID,
            title,
            text,
            self.core.settings().silent_notifications,
        );
    }

    /// The menu entries above the usual tray menu: the download item once a newer
    /// version is known.
    pub(crate) fn update_entries(&self) -> Vec<MenuEntry> {
        if self.new_version.is_none() {
            return Vec::new();
        }
        let label = self.core.words().text("main.downloadLatestVersion");
        vec![
            MenuEntry::Item {
                label,
                action: MenuAction::DownloadUpdate,
            },
            MenuEntry::Separator,
        ]
    }
}

/// Whether release tag `tag` (`v1.2.3` or `1.2.3`) is a later version than `current`.
fn newer(current: &str, tag: &str) -> bool {
    let parts = |version: &str| -> Option<Vec<u64>> {
        let version = version.trim().trim_start_matches('v');
        let core = version.split(['-', '+']).next()?;
        core.split('.').map(|part| part.parse().ok()).collect()
    };
    match (parts(current), parts(tag)) {
        (Some(current), Some(tag)) => tag > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::newer;

    #[test]
    fn only_a_later_release_tag_is_newer() {
        assert!(newer("0.1.0", "v0.2.0"));
        assert!(newer("0.1.0", "0.1.1"));
        assert!(newer("0.9.0", "v0.10.0"));
        assert!(!newer("0.1.0", "v0.1.0"));
        assert!(!newer("0.2.0", "v0.1.9"));
        assert!(!newer("0.1.0", "nightly"));
        assert!(!newer("0.1.0", "v0.1.0-rc.1"));
    }
}
