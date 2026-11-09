//! The tray menu, as upstream's `getTrayMenuTemplate`, without Preferences (needs a window)
//! and without shortcut labels (global hotkeys are not ported).

use breaks::{BreaksPlanner, Event};
use i18n::Translator;

use super::status;

/// What a tray menu item asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Resume,
    /// Pause for this many ms (`pauseBreaksDurations`).
    Pause(i64),
    PauseUntilMorning,
    PauseIndefinitely,
    SkipToMini,
    SkipToLong,
    Reset,
    Quit,
}

const PAUSE_PREFIX: &str = "pause-for-";

impl MenuAction {
    /// The menu item id, so a typo cannot silently disable an item.
    pub fn id(self) -> String {
        let id = match self {
            Self::Pause(duration) => return format!("{PAUSE_PREFIX}{duration}"),
            Self::Resume => "resume",
            Self::PauseUntilMorning => "pause-morning",
            Self::PauseIndefinitely => "pause-forever",
            Self::SkipToMini => "skip-mini",
            Self::SkipToLong => "skip-long",
            Self::Reset => "reset",
            Self::Quit => "quit",
        };
        id.to_owned()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        if let Some(duration) = id.strip_prefix(PAUSE_PREFIX) {
            return duration.parse().ok().map(Self::Pause);
        }
        [
            Self::Resume,
            Self::PauseUntilMorning,
            Self::PauseIndefinitely,
            Self::SkipToMini,
            Self::SkipToLong,
            Self::Reset,
            Self::Quit,
        ]
        .into_iter()
        .find(|action| action.id() == id)
    }
}

/// One line of the tray menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Item {
        label: String,
        action: MenuAction,
    },
    /// A status line that cannot be clicked.
    Text(String),
    Submenu {
        label: String,
        entries: Vec<MenuEntry>,
    },
    Separator,
}

/// The tray menu for the planner's state.
pub(super) fn tray_menu(planner: &BreaksPlanner, words: &Translator, now: i64) -> Vec<MenuEntry> {
    let s = planner.settings();
    let item = |action, key: &str| MenuEntry::Item {
        label: words.text(key),
        action,
    };
    let mut menu = Vec::new();
    if in_strict_break(planner) && !s.show_tray_menu_in_strict_mode {
        return menu;
    }
    let message = status::tray_message(planner, words, now);
    if !message.is_empty() {
        menu.extend(message.lines().map(|line| MenuEntry::Text(line.to_owned())));
        menu.push(MenuEntry::Separator);
    }
    let held = planner.dnd() || planner.excluded();
    if !(planner.paused() || held) && (s.mini_break || s.long_break) {
        let mut entries = Vec::new();
        if s.mini_break {
            entries.push(item(MenuAction::SkipToMini, "main.toMiniBreak"));
        }
        if s.long_break {
            entries.push(item(MenuAction::SkipToLong, "main.toLongBreak"));
        }
        menu.push(MenuEntry::Submenu {
            label: words.text("main.skipToTheNext"),
            entries,
        });
    }
    if planner.paused() {
        menu.push(item(MenuAction::Resume, "main.resume"));
    } else if !held {
        let mut entries: Vec<MenuEntry> = s
            .pause_durations()
            .map(|entry| MenuEntry::Item {
                label: duration_label(words, entry.duration),
                action: MenuAction::Pause(entry.duration),
            })
            .collect();
        entries.extend([
            item(MenuAction::PauseUntilMorning, "main.untilMorning"),
            MenuEntry::Separator,
            item(MenuAction::PauseIndefinitely, "main.indefinitely"),
        ]);
        menu.push(MenuEntry::Submenu {
            label: words.text("main.pause"),
            entries,
        });
        menu.push(item(MenuAction::Reset, "main.resetBreaks"));
    }
    menu.push(MenuEntry::Separator);
    menu.push(item(MenuAction::Quit, "main.quitStretchly"));
    menu
}

/// "30 minutes" or "2 hours" (upstream `pauseMenu`).
fn duration_label(words: &Translator, duration: i64) -> String {
    let minutes = (duration / 60_000) as u64;
    if minutes.is_multiple_of(60) {
        words.count("utils.hours", minutes / 60)
    } else {
        words.count("utils.minutes", minutes)
    }
}

/// A break runs in strict mode: upstream empties the tray menu and refuses to quit.
pub(super) fn in_strict_break(planner: &BreaksPlanner) -> bool {
    let s = planner.settings();
    match planner.next() {
        Event::FinishMiniBreak => s.mini_break_strict_mode,
        Event::FinishLongBreak => s.long_break_strict_mode,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings::Settings;

    fn planner(extra_json: &str) -> BreaksPlanner {
        let settings = Settings::from_json(&format!("{{{extra_json}}}")).unwrap();
        BreaksPlanner::new(settings, 0)
    }

    fn labels(entries: &[MenuEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| match entry {
                MenuEntry::Item { label, .. } | MenuEntry::Text(label) => label.clone(),
                MenuEntry::Submenu { label, entries } => {
                    format!("{label} [{}]", labels(entries).join(", "))
                }
                MenuEntry::Separator => "-".to_owned(),
            })
            .collect()
    }

    #[test]
    fn menu_follows_the_planner_state() {
        let words = Translator::new("en");
        let durations = r#""pauseBreaksDurations":[{"duration":1800000},{"duration":7200000}]"#;
        let mut running = planner(durations);
        assert_eq!(
            labels(&tray_menu(&running, &words, 0)),
            [
                "Next Mini break in about 10 minutes",
                "Next Long break after 2 Mini breaks",
                "-",
                "Skip to the next [Mini break, Long break]",
                "Pause breaks [30 minutes, 2 hours, Until morning, -, Indefinitely]",
                "Reset breaks",
                "-",
                "Quit Stretch",
            ]
        );
        running.pause(0, BreaksPlanner::INDEFINITELY);
        assert_eq!(
            labels(&tray_menu(&running, &words, 0)),
            [
                "Paused Indefinitely",
                "-",
                "Resume breaks",
                "-",
                "Quit Stretch"
            ]
        );
    }

    #[test]
    fn strict_break_empties_the_menu_unless_kept() {
        let words = Translator::new("en");
        let strict = r#""miniBreakStrictMode":true,"miniBreakNotification":false"#;
        let mut in_break = planner(strict);
        in_break.advance(600_000);
        assert!(in_strict_break(&in_break));
        assert!(tray_menu(&in_break, &words, 600_000).is_empty());
        let mut kept = planner(&format!("{strict},\"showTrayMenuInStrictMode\":true"));
        kept.advance(600_000);
        assert!(!tray_menu(&kept, &words, 600_000).is_empty());
    }

    #[test]
    fn every_menu_id_leads_back_to_its_own_action() {
        let actions = [
            MenuAction::Resume,
            MenuAction::Pause(1_800_000),
            MenuAction::PauseUntilMorning,
            MenuAction::PauseIndefinitely,
            MenuAction::SkipToMini,
            MenuAction::SkipToLong,
            MenuAction::Reset,
            MenuAction::Quit,
        ];
        for action in actions {
            assert_eq!(MenuAction::from_id(&action.id()), Some(action));
        }
    }
}
