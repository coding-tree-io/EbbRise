//! Puts Stretchly's translations into the Settings window's `Words`, for the labels whose
//! Stretchly text means the same. English and labels without such a text keep EbbRise's wording.

use i18n::Translator;
use slint::{ComponentHandle, SharedString};

use crate::ui::{SettingsWindow, Words};

type Setter<'w> = fn(&Words<'w>, SharedString);

/// The labels without a hint: each setter and Stretchly's key.
fn labels<'w>() -> [(Setter<'w>, &'static str); 15] {
    [
        (Words::set_general, "preferences.nav.settings"),
        (Words::set_schedule, "preferences.nav.schedule"),
        (Words::set_sounds, "preferences.theme.sounds"),
        (Words::set_about, "preferences.nav.about"),
        (
            Words::set_show_breaks_as,
            "preferences.settings.showBreaksIn",
        ),
        (Words::set_window, "preferences.settings.window"),
        (Words::set_full_screen, "preferences.settings.fullscreen"),
        (
            Words::set_start_at_login,
            "preferences.settings.openAtLogin",
        ),
        (Words::set_language, "preferences.settings.language"),
        (Words::set_theme, "preferences.nav.theme"),
        (Words::set_every, "preferences.schedule.every"),
        (Words::set_for_length, "preferences.schedule.breakFor"),
        (Words::set_version, "preferences.about.version"),
        (Words::set_tagline, "preferences.about.tagline"),
        (
            Words::set_restore_defaults,
            "preferences.settings.restoreDefaults",
        ),
    ]
}

/// The row titles with a hint under them: setter, Stretchly's key and the hint's setter.
fn titles<'w>() -> [(Setter<'w>, &'static str, Setter<'w>); 7] {
    [
        (
            Words::set_show_ideas,
            "preferences.settings.showIdeas",
            Words::set_show_ideas_hint,
        ),
        (
            Words::set_take_mini,
            "preferences.schedule.enableMiniBreaks",
            Words::set_take_mini_hint,
        ),
        (
            Words::set_take_long,
            "preferences.schedule.enableLongBreaks",
            Words::set_take_long_hint,
        ),
        (
            Words::set_notify_mini,
            "preferences.schedule.showNotificationBeforeMiniBreak",
            Words::set_notify_mini_hint,
        ),
        (
            Words::set_notify_long,
            "preferences.schedule.showNotificationBeforeLongBreak",
            Words::set_notify_long_hint,
        ),
        (
            Words::set_strict_mini,
            "preferences.schedule.enableStrictMini",
            Words::set_strict_mini_hint,
        ),
        (
            Words::set_strict_long,
            "preferences.schedule.enableStrictLong",
            Words::set_strict_long_hint,
        ),
    ]
}

/// The section labels, shown in capitals as the English ones.
fn sections<'w>() -> [(Setter<'w>, &'static str); 3] {
    [
        (Words::set_mini_breaks, "preferences.schedule.miniBreaks"),
        (Words::set_long_breaks, "preferences.schedule.longBreaks"),
        (Words::set_appearance, "preferences.theme.appearance"),
    ]
}

/// Sets every label `words` has a translation for. Run once, when the window opens; a
/// language change opens the window again.
pub(crate) fn translate(window: &SettingsWindow, words: &Translator) {
    let global = window.global::<Words>();
    for (set, key) in labels() {
        if let Some(text) = words.translated(key) {
            set(&global, label(&text).into());
        }
    }
    // A translated title clears its English hint, so a row never mixes two languages.
    for (set, key, clear_hint) in titles() {
        if let Some(text) = words.translated(key) {
            set(&global, label(&text).into());
            clear_hint(&global, SharedString::new());
        }
    }
    for (set, key) in sections() {
        if let Some(text) = words.translated(key) {
            set(&global, capitals(&label(&text)).into());
        }
    }
}

/// `text` in capitals. Greek capitals drop the accent (ά becomes Α, not Ά).
fn capitals(text: &str) -> String {
    text.to_uppercase()
        .chars()
        .map(|c| match c {
            'Ά' => 'Α',
            'Έ' => 'Ε',
            'Ή' => 'Η',
            'Ί' => 'Ι',
            'Ό' => 'Ο',
            'Ύ' => 'Υ',
            'Ώ' => 'Ω',
            other => other,
        })
        .collect()
}

/// Stretchly's form labels end with a colon ("Every:"); EbbRise's rows do not.
fn label(text: &str) -> String {
    text.trim()
        .trim_end_matches([':', '：'])
        .trim_end()
        .to_owned()
}

/// The display name of a sound: Stretchly's translated name for a built-in one.
pub(crate) fn sound_name(words: &Translator, name: &str) -> Option<String> {
    let key = match name {
        "crystal-glass" => "preferences.theme.crystalGlass",
        "wind-chime" => "preferences.theme.windChime",
        "tic-toc" => "preferences.theme.ticToc",
        _ => return None,
    };
    Some(words.text(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_lose_the_colon_and_greek_capitals_lose_the_accent() {
        assert_eq!(label("Every: "), "Every");
        assert_eq!(label("毎："), "毎");
        assert_eq!(capitals("Μικρά διαλείμματα"), "ΜΙΚΡΑ ΔΙΑΛΕΙΜΜΑΤΑ");
        assert_eq!(capitals("Mini-Pausen"), "MINI-PAUSEN");
    }
}
