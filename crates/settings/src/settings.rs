use serde::{Deserialize, Serialize};

/// When "until morning" ends (`morningHour`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MorningHour {
    /// Local hour, 0–23.
    Hour(i64),
    /// Local sunrise at [`Settings::pos_latitude`] and [`Settings::pos_longitude`] when the
    /// text is `"sunrise"`; any other text means no morning time.
    Sunrise(String),
}

/// A break idea that the user can turn off.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Idea<T> {
    /// Text of a Mini break idea, or `[title, text]` of a Long break idea.
    pub data: T,
    /// Whether the idea may be picked.
    pub enabled: bool,
}

/// One app exclusion rule (`appExclusions`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppExclusion {
    /// `"pause"`: no breaks while a command runs. `"resume"`: breaks only while one runs.
    pub rule: String,
    /// Only the first active rule applies.
    pub active: bool,
    /// Process names, matched by substring.
    pub commands: Vec<String>,
}

/// A pause length offered in the tray menu (`pauseBreaksDurations`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PauseDuration {
    /// Milliseconds.
    pub duration: i64,
    /// Global shortcut in Electron accelerator format; empty for none.
    pub shortcut: String,
}

/// Stretchly preferences, with the same JSON keys (camelCase) as Stretchly's `config.json`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Mini break length, ms.
    pub mini_break_duration: i64,
    /// Time between breaks, ms.
    pub mini_break_interval: i64,
    /// Long break length, ms.
    pub long_break_duration: i64,
    /// Mini breaks before each Long break; every (this + 1)-th break is Long.
    pub long_break_interval: i64,
    /// Warn before a Long break.
    pub long_break_notification: bool,
    /// Warn before a Mini break.
    pub mini_break_notification: bool,
    /// How long before a Long break the warning comes, ms.
    pub long_break_notification_interval: i64,
    /// How long before a Mini break the warning comes, ms.
    pub mini_break_notification_interval: i64,
    /// Mini breaks are on.
    pub mini_break: bool,
    /// Long breaks are on.
    pub long_break: bool,
    /// Mini breaks cannot be skipped or postponed.
    pub mini_break_strict_mode: bool,
    /// Long breaks cannot be skipped or postponed.
    pub long_break_strict_mode: bool,
    /// When "pause until morning" ends.
    pub morning_hour: MorningHour,
    /// Mini breaks can be postponed.
    pub mini_break_postpone: bool,
    /// Long breaks can be postponed.
    pub long_break_postpone: bool,
    /// How far a postpone moves a Mini break, ms.
    pub mini_break_postpone_time: i64,
    /// How far a postpone moves a Long break, ms.
    pub long_break_postpone_time: i64,
    /// Postpones allowed per Mini break.
    pub mini_break_postpones_limit: u32,
    /// Share of a Mini break (0–100) after which postponing is no longer offered.
    pub mini_break_postponable_duration_percent: i64,
    /// Postpones allowed per Long break.
    pub long_break_postpones_limit: u32,
    /// Share of a Long break (0–100) after which postponing is no longer offered.
    pub long_break_postponable_duration_percent: i64,
    /// Long break window colour, `#rrggbb`.
    pub long_break_color: String,
    /// Mini break window colour, `#rrggbb`.
    pub mini_break_color: String,
    /// Break windows use [`Settings::opacity`].
    pub transparent_mode: bool,
    /// Break windows blur what is behind them (macOS only upstream).
    pub blurred_background: bool,
    /// Break window opacity in transparent mode, 0–1.
    pub opacity: f64,
    /// Sound at the end of a Long break: a bundled name or a custom WAV file name.
    pub long_break_audio: String,
    /// Sound at the end of a Mini break: a bundled name or a custom WAV file name.
    pub mini_break_audio: String,
    /// Sound volume, 0–1.
    pub volume: f64,
    /// Break windows cover the whole display.
    pub fullscreen: bool,
    /// Show break ideas in break windows.
    pub ideas: bool,
    /// Idle time counts as a break (natural breaks).
    pub natural_breaks: bool,
    /// Idle time after which the break cycle restarts, ms.
    pub natural_breaks_inactivity_reset_time: i64,
    /// Upstream idle poll interval, ms; the port checks on demand instead.
    pub natural_breaks_check_interval: i64,
    /// Show break windows on every display.
    pub all_screens: bool,
    /// Use [`Settings::mini_break_ideas`] and [`Settings::long_break_ideas`] instead of the built-in ideas.
    pub use_ideas_from_settings: bool,
    /// UI language code, such as `en` or `pt-BR`.
    pub language: String,
    /// Tell the user about a new version.
    pub notify_new_version: bool,
    /// The welcome window has not been shown yet.
    pub is_first_run: bool,
    /// Latitude for sunrise, degrees.
    pub pos_latitude: f64,
    /// Longitude for sunrise, degrees.
    pub pos_longitude: f64,
    /// Use the monochrome tray icon.
    pub use_monochrome_tray_icon: bool,
    /// Tray icon theme: `system`, `light` or `dark`.
    pub tray_icon_theme_source: String,
    /// Notifications make no sound.
    pub silent_notifications: bool,
    /// Do Not Disturb (Focus Assist) holds breaks while it is on.
    pub monitor_dnd: bool,
    /// Upstream DND poll interval, ms; the port gets changes pushed.
    pub monitor_dnd_check_interval: i64,
    /// Sound at the start of a Mini break; `silence` for none.
    pub mini_break_start_sound: String,
    /// Sound at the start of a Long break; `silence` for none.
    pub long_break_start_sound: String,
    /// Window theme: `system`, `light` or `dark`.
    pub theme_source: String,
    /// Global shortcut that ends the current break, Electron accelerator format.
    pub end_break_shortcut: String,
    /// Break window width as a share of the display, 0–1.
    pub break_window_width: f64,
    /// Break window height as a share of the display, 0–1.
    pub break_window_height: f64,
    /// Check for a new version at start.
    pub check_new_version: bool,
    /// Custom Long break ideas.
    pub long_break_ideas: Vec<Idea<Vec<String>>>,
    /// Custom Mini break ideas.
    pub mini_break_ideas: Vec<Idea<String>>,
    /// Break windows are normal windows, not top-most overlays.
    pub show_breaks_as_regular_windows: bool,
    /// App exclusion rules; the first active one applies.
    pub app_exclusions: Vec<AppExclusion>,
    /// How often to check running processes for app exclusions, ms.
    pub app_exclusions_check_interval: i64,
    /// Pause breaks while the PC sleeps or is locked, and restart them after.
    pub pause_for_suspend_or_lock: bool,
    /// Pause lengths offered in the tray menu.
    pub pause_breaks_durations: Vec<PauseDuration>,
    /// Global shortcut that pauses or resumes breaks; empty for none.
    pub pause_breaks_toggle_shortcut: String,
    /// Global shortcut that pauses breaks until morning; empty for none.
    pub pause_breaks_until_morning_shortcut: String,
    /// Display for break windows when not on all screens: `primary`, `cursor` or an index as text.
    pub screen: String,
    /// Display that shows the break content: `all`, `primary`, `cursor` or an index as text.
    pub break_content_screen: String,
    /// Tray icon style: `default`, `progress` or `time`.
    pub tray_icon_style: String,
    /// Show the current time in break windows.
    pub current_time_in_breaks: bool,
    /// Font size of that time, px.
    pub current_time_in_breaks_font_size: i64,
    /// Luxon format of that time, such as `tt`.
    pub current_time_in_breaks_format: String,
    /// Show the tray icon.
    pub show_tray_icon: bool,
    /// Global shortcut that starts the next scheduled break now; empty for none.
    pub skip_to_next_scheduled_break_shortcut: String,
    /// Global shortcut that starts a Mini break now; empty for none.
    pub skip_to_next_mini_break_shortcut: String,
    /// Global shortcut that starts a Long break now; empty for none.
    pub skip_to_next_long_break_shortcut: String,
    /// Global shortcut that resets breaks; empty for none.
    pub reset_breaks_shortcut: String,
    /// Global shortcut that repeats the last break; empty for none.
    pub start_last_break_again_shortcut: String,
    /// Keep the tray menu in strict mode.
    pub show_tray_menu_in_strict_mode: bool,
    /// Text shown at the top of Preferences, for managed installs.
    pub custom_preferences_message: String,
    /// Hide update checks (Store and portable builds).
    pub disable_app_update_features: bool,
    /// Hide the preferences file path in Preferences.
    pub hide_preferences_file_location: bool,
    /// Hide the strict mode options in Preferences.
    pub hide_strict_mode_preferences: bool,
    /// A Mini break waits for the user after its time ends.
    pub mini_break_manual_finish: bool,
    /// A Long break waits for the user after its time ends.
    pub long_break_manual_finish: bool,
    /// Show break health (danger) from skips and postpones.
    pub break_health_mode: bool,
    /// Start at login.
    pub open_at_login: bool,
    /// Linux only; kept for config compatibility.
    pub force_x11_on_wayland: bool,
    /// Upstream one-time autostart migration is done.
    #[serde(rename = "_migratedOpenAtLogin")]
    pub migrated_open_at_login: bool,
    /// Keys this version does not know, such as ones from a newer version; saved back as they are.
    #[serde(flatten)]
    pub unknown_keys: serde_json::Map<String, serde_json::Value>,
}

const DEFAULTS: &str = include_str!("default-settings.json");

impl Default for Settings {
    /// Stretchly 1.22.1 defaults on Windows (`defaultSettings.js`).
    fn default() -> Self {
        serde_json::from_str(DEFAULTS).expect("valid bundled defaults")
    }
}

impl Settings {
    /// Reads a Stretchly-style config; top-level keys it lacks keep their defaults.
    ///
    /// This is a shallow merge, like electron-store's `Object.assign(defaults, store)`.
    ///
    /// # Errors
    /// When `text` is not a JSON object or a value has the wrong type.
    ///
    /// # Panics
    /// Only if the bundled defaults are not valid JSON, which the tests rule out.
    pub fn from_json(text: &str) -> serde_json::Result<Self> {
        type Object = serde_json::Map<String, serde_json::Value>;
        let mut settings: Object = serde_json::from_str(DEFAULTS).expect("valid bundled defaults");
        settings.extend(serde_json::from_str::<Object>(text)?);
        serde_json::from_value(settings.into())
    }
}
