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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsForm {
    pub fullscreen: bool,
    pub ideas: bool,
    pub natural_breaks: bool,
    pub open_at_login: bool,
    pub monitor_dnd: bool,
    /// Index into [`LANGUAGES`].
    pub language: usize,
    /// Index into [`THEMES`]: follow Windows, light or dark.
    pub theme: usize,
    pub mini: BreakForm,
    pub long: BreakForm,
    /// Sound names; the first is [`SILENCE`].
    pub sounds: Vec<String>,
    /// Indexes into `sounds`: Mini start, Mini end, Long start, Long end.
    pub sound_choice: [usize; 4],
    pub volume: i64,
}

const MINUTE: i64 = 60_000;
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
            sounds,
            sound_choice,
            volume: (s.volume * 100.0).round() as i64,
        }
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
        if self.language != before.language {
            s.language = LANGUAGES[self.language].0.to_owned();
        }
        if self.theme != before.theme {
            s.theme_source = THEMES[self.theme.min(THEMES.len() - 1)].to_owned();
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
