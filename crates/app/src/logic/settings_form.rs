//! The settings window's values, taken from [`Settings`] and written back.
//!
//! Units follow the window: Mini break every N minutes for N seconds, Long break after N Mini
//! breaks for N minutes, volume in percent. Writing back changes only the fields the user
//! changed, so a value the window cannot show exactly (such as a 90-second interval) survives.

use i18n::LANGUAGES;
use settings::Settings;

use super::SILENCE;

/// One break kind's controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakForm {
    pub on: bool,
    /// Mini: minutes between breaks. Long: Mini breaks before each Long break.
    pub every: i64,
    /// Mini: seconds. Long: minutes.
    pub length: i64,
    pub notify: bool,
    pub strict: bool,
}

/// The controls for the features from Workrave (`docs/workrave-features.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsageForm {
    pub active_time_only: bool,
    pub block_input: bool,
    pub daily_limit: bool,
    /// Hours of active time per day.
    pub daily_limit_hours: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsForm {
    pub fullscreen: bool,
    pub ideas: bool,
    pub natural_breaks: bool,
    pub open_at_login: bool,
    pub monitor_dnd: bool,
    pub no_breaks_in_full_screen: bool,
    pub pause_media: bool,
    /// Stretchly's `screen`: "primary", "cursor" or a display index.
    pub screen: String,
    pub all_screens: bool,
    /// Index into [`LANGUAGES`].
    pub language: usize,
    /// Index into [`THEMES`]: follow Windows, light or dark.
    pub theme: usize,
    pub mini: BreakForm,
    pub long: BreakForm,
    pub usage: UsageForm,
    /// Sound names; the first is [`SILENCE`].
    pub sounds: Vec<String>,
    /// Indexes into `sounds`: Mini start, Mini end, Long start, Long end.
    pub sound_choice: [usize; 4],
    pub volume: i64,
}

const MINUTE: i64 = 60_000;
const HOUR: i64 = 3_600_000;
/// Stretchly's `themeSource` values, in the order of the window's theme switch.
const THEMES: [&str; 3] = ["system", "light", "dark"];

impl SettingsForm {
    /// `sound_files` are the names of the WAV files in the audio folder.
    pub fn new(s: &Settings, sound_files: &[String]) -> Self {
        let mut sounds = vec![SILENCE.to_owned()];
        sounds.extend(sound_files.iter().cloned());
        let current = [
            &s.mini_break_start_sound,
            &s.mini_break_audio,
            &s.long_break_start_sound,
            &s.long_break_audio,
        ];
        // A name without a file stays selectable, so opening the window changes nothing.
        for name in current {
            if !sounds.contains(name) {
                sounds.push(name.clone());
            }
        }
        let index = |name: &String| sounds.iter().position(|sound| sound == name).unwrap_or(0);
        let sound_choice = current.map(index);
        let language = |code: &str| LANGUAGES.iter().position(|(c, _)| *c == code);
        Self {
            fullscreen: s.fullscreen,
            ideas: s.ideas,
            natural_breaks: s.natural_breaks,
            open_at_login: s.open_at_login,
            monitor_dnd: s.monitor_dnd,
            no_breaks_in_full_screen: s.no_breaks_in_full_screen,
            pause_media: s.pause_media_during_breaks,
            screen: s.screen.clone(),
            all_screens: s.all_screens,
            language: language(&s.language)
                .or_else(|| language("en"))
                .unwrap_or(0),
            theme: THEMES
                .iter()
                .position(|theme| *theme == s.theme_source)
                .unwrap_or(0),
            mini: BreakForm {
                on: s.mini_break,
                every: (s.mini_break_interval / MINUTE).max(1),
                length: (s.mini_break_duration / 1000).max(1),
                notify: s.mini_break_notification,
                strict: s.mini_break_strict_mode,
            },
            long: BreakForm {
                on: s.long_break,
                every: (s.long_break_every / MINUTE).max(1),
                length: (s.long_break_duration / MINUTE).max(1),
                notify: s.long_break_notification,
                strict: s.long_break_strict_mode,
            },
            usage: UsageForm {
                active_time_only: s.count_active_time_only,
                block_input: s.block_input_during_breaks,
                daily_limit: s.daily_limit,
                daily_limit_hours: (s.daily_limit_time / HOUR).max(1),
            },
            sounds,
            sound_choice,
            volume: (s.volume * 100.0).round() as i64,
        }
    }

    /// The "When you're away from the PC" choice: 0 keeps counting, 1 pauses the timer
    /// (only active time counts), 2 counts as a break (natural breaks, with or without
    /// only active time: `docs/workrave-features.md`, decision D1).
    pub fn away(&self) -> usize {
        match (self.natural_breaks, self.usage.active_time_only) {
            (true, _) => 2,
            (false, true) => 1,
            (false, false) => 0,
        }
    }

    /// Sets the two settings behind [`Self::away`] when the choice changes.
    pub fn set_away(&mut self, choice: usize) {
        if choice == self.away() {
            return;
        }
        self.natural_breaks = choice == 2;
        self.usage.active_time_only = choice == 1;
    }

    /// The display names of `sounds`: "None" for silence.
    pub fn sound_labels(&self) -> Vec<String> {
        let label = |name: &String| {
            if name == SILENCE {
                "None".to_owned()
            } else {
                name.clone()
            }
        };
        self.sounds.iter().map(label).collect()
    }

    /// `base` with the fields this form changed from `base`'s own form.
    pub fn apply(&self, base: &Settings) -> Settings {
        let before = Self::new(base, &self.sounds[1..]);
        let mut s = base.clone();
        if self.fullscreen != before.fullscreen {
            s.fullscreen = self.fullscreen;
        }
        if self.ideas != before.ideas {
            s.ideas = self.ideas;
        }
        if self.natural_breaks != before.natural_breaks {
            s.natural_breaks = self.natural_breaks;
        }
        if self.open_at_login != before.open_at_login {
            s.open_at_login = self.open_at_login;
        }
        if self.monitor_dnd != before.monitor_dnd {
            s.monitor_dnd = self.monitor_dnd;
        }
        s.no_breaks_in_full_screen = self.no_breaks_in_full_screen;
        s.pause_media_during_breaks = self.pause_media;
        s.all_screens = self.all_screens;
        if self.screen != before.screen {
            s.screen = self.screen.clone();
        }
        if self.language != before.language {
            s.language = LANGUAGES[self.language].0.to_owned();
        }
        if self.theme != before.theme {
            s.theme_source = THEMES[self.theme.min(THEMES.len() - 1)].to_owned();
        }
        s.count_active_time_only = self.usage.active_time_only;
        s.block_input_during_breaks = self.usage.block_input;
        s.daily_limit = self.usage.daily_limit;
        if self.usage.daily_limit_hours != before.usage.daily_limit_hours {
            s.daily_limit_time = self.usage.daily_limit_hours * HOUR;
        }
        apply_mini(&self.mini, &before.mini, &mut s);
        apply_long(&self.long, &before.long, &mut s);
        let slots = [
            &mut s.mini_break_start_sound,
            &mut s.mini_break_audio,
            &mut s.long_break_start_sound,
            &mut s.long_break_audio,
        ];
        for ((slot, now), was) in slots
            .into_iter()
            .zip(self.sound_choice)
            .zip(before.sound_choice)
        {
            if now != was {
                *slot = self.sounds[now].clone();
            }
        }
        if self.volume != before.volume {
            s.volume = self.volume.clamp(0, 100) as f64 / 100.0;
        }
        s
    }
}

fn apply_mini(form: &BreakForm, before: &BreakForm, s: &mut Settings) {
    s.mini_break = form.on;
    s.mini_break_notification = form.notify;
    s.mini_break_strict_mode = form.strict;
    if form.every != before.every {
        s.mini_break_interval = form.every * MINUTE;
    }
    if form.length != before.length {
        s.mini_break_duration = form.length * 1000;
    }
}

fn apply_long(form: &BreakForm, before: &BreakForm, s: &mut Settings) {
    s.long_break = form.on;
    s.long_break_notification = form.notify;
    s.long_break_strict_mode = form.strict;
    if form.every != before.every {
        s.long_break_every = form.every * MINUTE;
    }
    if form.length != before.length {
        s.long_break_duration = form.length * MINUTE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        vec!["wind-chime".to_owned()]
    }

    #[test]
    fn an_unchanged_form_changes_nothing_even_for_values_it_rounds() {
        let odd = Settings {
            mini_break_interval: 90_000,
            mini_break_audio: "missing-file".to_owned(),
            ..Settings::default()
        };
        let form = SettingsForm::new(&odd, &files());
        assert_eq!(form.mini.every, 1);
        // The default Long break end sound, crystal-glass, has no file either.
        assert_eq!(
            form.sounds,
            ["silence", "wind-chime", "missing-file", "crystal-glass"]
        );
        assert_eq!(form.apply(&odd), odd);
    }

    #[test]
    fn the_away_choice_maps_exact_pairs_and_preserves_unchanged_legacy_settings() {
        // (starting pair, choice, expected pair)
        let cases = [
            ((false, true), 0, (false, false)),
            ((true, false), 1, (false, true)),
            ((false, true), 2, (true, false)),
            ((true, true), 2, (true, true)),
        ];
        for ((natural, active), choice, expected) in cases {
            let start = Settings {
                natural_breaks: natural,
                count_active_time_only: active,
                ..Settings::default()
            };
            let mut form = SettingsForm::new(&start, &files());
            form.set_away(choice);
            let s = form.apply(&start);
            assert_eq!((s.natural_breaks, s.count_active_time_only), expected);
            assert_eq!(SettingsForm::new(&s, &files()).away(), choice);
            if (natural, active) == (true, true) {
                assert_eq!(s, start, "an unchanged choice changes nothing");
            }
        }
    }

    #[test]
    fn changed_controls_write_their_settings_in_settings_units() {
        let base = Settings::default();
        let mut form = SettingsForm::new(&base, &files());
        form.mini.every = 25;
        form.long.every = 45;
        form.long.length = 7;
        form.sound_choice[1] = 1;
        form.volume = 45;
        form.theme = 2;
        form.language = LANGUAGES.iter().position(|(c, _)| *c == "el").unwrap();
        let s = form.apply(&base);
        assert_eq!(s.mini_break_interval, 25 * MINUTE);
        assert_eq!(s.long_break_every, 45 * MINUTE);
        assert_eq!(s.long_break_duration, 7 * MINUTE);
        assert_eq!(s.mini_break_audio, "wind-chime");
        assert_eq!((s.volume, s.language.as_str()), (0.45, "el"));
        assert_eq!(s.theme_source, "dark");
        assert_eq!(form.sound_labels()[0], "None");
    }
}
