use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MorningHour {
    Hour(i64),
    Sunrise(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Idea<T> {
    pub data: T,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppExclusion {
    pub rule: String,
    pub active: bool,
    pub commands: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PauseDuration {
    pub duration: i64,
    pub shortcut: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub mini_break_duration: i64,
    pub mini_break_interval: i64,
    pub long_break_duration: i64,
    pub long_break_interval: i64,
    pub long_break_notification: bool,
    pub mini_break_notification: bool,
    pub long_break_notification_interval: i64,
    pub mini_break_notification_interval: i64,
    pub mini_break: bool,
    pub long_break: bool,
    pub mini_break_strict_mode: bool,
    pub long_break_strict_mode: bool,
    pub morning_hour: MorningHour,
    pub mini_break_postpone: bool,
    pub long_break_postpone: bool,
    pub mini_break_postpone_time: i64,
    pub long_break_postpone_time: i64,
    pub mini_break_postpones_limit: u32,
    pub mini_break_postponable_duration_percent: i64,
    pub long_break_postpones_limit: u32,
    pub long_break_postponable_duration_percent: i64,
    pub long_break_color: String,
    pub mini_break_color: String,
    pub transparent_mode: bool,
    pub blurred_background: bool,
    pub opacity: f64,
    pub long_break_audio: String,
    pub mini_break_audio: String,
    pub volume: f64,
    pub fullscreen: bool,
    pub ideas: bool,
    pub natural_breaks: bool,
    pub natural_breaks_inactivity_reset_time: i64,
    pub natural_breaks_check_interval: i64,
    pub all_screens: bool,
    pub use_ideas_from_settings: bool,
    pub language: String,
    pub notify_new_version: bool,
    pub is_first_run: bool,
    pub pos_latitude: f64,
    pub pos_longitude: f64,
    pub use_monochrome_tray_icon: bool,
    pub tray_icon_theme_source: String,
    pub silent_notifications: bool,
    pub monitor_dnd: bool,
    pub monitor_dnd_check_interval: i64,
    pub mini_break_start_sound: String,
    pub long_break_start_sound: String,
    pub theme_source: String,
    pub end_break_shortcut: String,
    pub break_window_width: f64,
    pub break_window_height: f64,
    pub check_new_version: bool,
    pub long_break_ideas: Vec<Idea<Vec<String>>>,
    pub mini_break_ideas: Vec<Idea<String>>,
    pub show_breaks_as_regular_windows: bool,
    pub app_exclusions: Vec<AppExclusion>,
    pub app_exclusions_check_interval: i64,
    pub pause_for_suspend_or_lock: bool,
    pub pause_breaks_durations: Vec<PauseDuration>,
    pub pause_breaks_toggle_shortcut: String,
    pub pause_breaks_until_morning_shortcut: String,
    pub screen: String,
    pub break_content_screen: String,
    pub tray_icon_style: String,
    pub current_time_in_breaks: bool,
    pub current_time_in_breaks_font_size: i64,
    pub current_time_in_breaks_format: String,
    pub show_tray_icon: bool,
    pub skip_to_next_scheduled_break_shortcut: String,
    pub skip_to_next_mini_break_shortcut: String,
    pub skip_to_next_long_break_shortcut: String,
    pub reset_breaks_shortcut: String,
    pub start_last_break_again_shortcut: String,
    pub show_tray_menu_in_strict_mode: bool,
    pub custom_preferences_message: String,
    pub disable_app_update_features: bool,
    pub hide_preferences_file_location: bool,
    pub hide_strict_mode_preferences: bool,
    pub mini_break_manual_finish: bool,
    pub long_break_manual_finish: bool,
    pub break_health_mode: bool,
    pub open_at_login: bool,
    pub force_x11_on_wayland: bool,
    #[serde(rename = "_migratedOpenAtLogin")]
    pub migrated_open_at_login: bool,
}

impl Settings {
    pub fn windows_defaults(store: bool, portable: bool) -> Self {
        Self {
            disable_app_update_features: store || portable,
            ..Self::default()
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let defaults: serde_json::Value =
            serde_json::from_str(include_str!("default-settings.json"))
                .expect("valid bundled defaults");
        Self {
            mini_break_duration: serde_json::from_value(defaults["miniBreakDuration"].clone())
                .expect("valid bundled default"),
            mini_break_interval: serde_json::from_value(defaults["miniBreakInterval"].clone())
                .expect("valid bundled default"),
            long_break_duration: serde_json::from_value(defaults["longBreakDuration"].clone())
                .expect("valid bundled default"),
            long_break_interval: serde_json::from_value(defaults["longBreakInterval"].clone())
                .expect("valid bundled default"),
            long_break_notification: serde_json::from_value(
                defaults["longBreakNotification"].clone(),
            )
            .expect("valid bundled default"),
            mini_break_notification: serde_json::from_value(
                defaults["miniBreakNotification"].clone(),
            )
            .expect("valid bundled default"),
            long_break_notification_interval: serde_json::from_value(
                defaults["longBreakNotificationInterval"].clone(),
            )
            .expect("valid bundled default"),
            mini_break_notification_interval: serde_json::from_value(
                defaults["miniBreakNotificationInterval"].clone(),
            )
            .expect("valid bundled default"),
            mini_break: serde_json::from_value(defaults["miniBreak"].clone())
                .expect("valid bundled default"),
            long_break: serde_json::from_value(defaults["longBreak"].clone())
                .expect("valid bundled default"),
            mini_break_strict_mode: serde_json::from_value(defaults["miniBreakStrictMode"].clone())
                .expect("valid bundled default"),
            long_break_strict_mode: serde_json::from_value(defaults["longBreakStrictMode"].clone())
                .expect("valid bundled default"),
            morning_hour: serde_json::from_value(defaults["morningHour"].clone())
                .expect("valid bundled default"),
            mini_break_postpone: serde_json::from_value(defaults["miniBreakPostpone"].clone())
                .expect("valid bundled default"),
            long_break_postpone: serde_json::from_value(defaults["longBreakPostpone"].clone())
                .expect("valid bundled default"),
            mini_break_postpone_time: serde_json::from_value(
                defaults["miniBreakPostponeTime"].clone(),
            )
            .expect("valid bundled default"),
            long_break_postpone_time: serde_json::from_value(
                defaults["longBreakPostponeTime"].clone(),
            )
            .expect("valid bundled default"),
            mini_break_postpones_limit: serde_json::from_value(
                defaults["miniBreakPostponesLimit"].clone(),
            )
            .expect("valid bundled default"),
            mini_break_postponable_duration_percent: serde_json::from_value(
                defaults["miniBreakPostponableDurationPercent"].clone(),
            )
            .expect("valid bundled default"),
            long_break_postpones_limit: serde_json::from_value(
                defaults["longBreakPostponesLimit"].clone(),
            )
            .expect("valid bundled default"),
            long_break_postponable_duration_percent: serde_json::from_value(
                defaults["longBreakPostponableDurationPercent"].clone(),
            )
            .expect("valid bundled default"),
            long_break_color: serde_json::from_value(defaults["longBreakColor"].clone())
                .expect("valid bundled default"),
            mini_break_color: serde_json::from_value(defaults["miniBreakColor"].clone())
                .expect("valid bundled default"),
            transparent_mode: serde_json::from_value(defaults["transparentMode"].clone())
                .expect("valid bundled default"),
            blurred_background: serde_json::from_value(defaults["blurredBackground"].clone())
                .expect("valid bundled default"),
            opacity: serde_json::from_value(defaults["opacity"].clone())
                .expect("valid bundled default"),
            long_break_audio: serde_json::from_value(defaults["longBreakAudio"].clone())
                .expect("valid bundled default"),
            mini_break_audio: serde_json::from_value(defaults["miniBreakAudio"].clone())
                .expect("valid bundled default"),
            volume: serde_json::from_value(defaults["volume"].clone())
                .expect("valid bundled default"),
            fullscreen: serde_json::from_value(defaults["fullscreen"].clone())
                .expect("valid bundled default"),
            ideas: serde_json::from_value(defaults["ideas"].clone())
                .expect("valid bundled default"),
            natural_breaks: serde_json::from_value(defaults["naturalBreaks"].clone())
                .expect("valid bundled default"),
            natural_breaks_inactivity_reset_time: serde_json::from_value(
                defaults["naturalBreaksInactivityResetTime"].clone(),
            )
            .expect("valid bundled default"),
            natural_breaks_check_interval: serde_json::from_value(
                defaults["naturalBreaksCheckInterval"].clone(),
            )
            .expect("valid bundled default"),
            all_screens: serde_json::from_value(defaults["allScreens"].clone())
                .expect("valid bundled default"),
            use_ideas_from_settings: serde_json::from_value(
                defaults["useIdeasFromSettings"].clone(),
            )
            .expect("valid bundled default"),
            language: serde_json::from_value(defaults["language"].clone())
                .expect("valid bundled default"),
            notify_new_version: serde_json::from_value(defaults["notifyNewVersion"].clone())
                .expect("valid bundled default"),
            is_first_run: serde_json::from_value(defaults["isFirstRun"].clone())
                .expect("valid bundled default"),
            pos_latitude: serde_json::from_value(defaults["posLatitude"].clone())
                .expect("valid bundled default"),
            pos_longitude: serde_json::from_value(defaults["posLongitude"].clone())
                .expect("valid bundled default"),
            use_monochrome_tray_icon: serde_json::from_value(
                defaults["useMonochromeTrayIcon"].clone(),
            )
            .expect("valid bundled default"),
            tray_icon_theme_source: serde_json::from_value(defaults["trayIconThemeSource"].clone())
                .expect("valid bundled default"),
            silent_notifications: serde_json::from_value(defaults["silentNotifications"].clone())
                .expect("valid bundled default"),
            monitor_dnd: serde_json::from_value(defaults["monitorDnd"].clone())
                .expect("valid bundled default"),
            monitor_dnd_check_interval: serde_json::from_value(
                defaults["monitorDndCheckInterval"].clone(),
            )
            .expect("valid bundled default"),
            mini_break_start_sound: serde_json::from_value(defaults["miniBreakStartSound"].clone())
                .expect("valid bundled default"),
            long_break_start_sound: serde_json::from_value(defaults["longBreakStartSound"].clone())
                .expect("valid bundled default"),
            theme_source: serde_json::from_value(defaults["themeSource"].clone())
                .expect("valid bundled default"),
            end_break_shortcut: serde_json::from_value(defaults["endBreakShortcut"].clone())
                .expect("valid bundled default"),
            break_window_width: serde_json::from_value(defaults["breakWindowWidth"].clone())
                .expect("valid bundled default"),
            break_window_height: serde_json::from_value(defaults["breakWindowHeight"].clone())
                .expect("valid bundled default"),
            check_new_version: serde_json::from_value(defaults["checkNewVersion"].clone())
                .expect("valid bundled default"),
            long_break_ideas: serde_json::from_value(defaults["longBreakIdeas"].clone())
                .expect("valid bundled default"),
            mini_break_ideas: serde_json::from_value(defaults["miniBreakIdeas"].clone())
                .expect("valid bundled default"),
            show_breaks_as_regular_windows: serde_json::from_value(
                defaults["showBreaksAsRegularWindows"].clone(),
            )
            .expect("valid bundled default"),
            app_exclusions: serde_json::from_value(defaults["appExclusions"].clone())
                .expect("valid bundled default"),
            app_exclusions_check_interval: serde_json::from_value(
                defaults["appExclusionsCheckInterval"].clone(),
            )
            .expect("valid bundled default"),
            pause_for_suspend_or_lock: serde_json::from_value(
                defaults["pauseForSuspendOrLock"].clone(),
            )
            .expect("valid bundled default"),
            pause_breaks_durations: serde_json::from_value(
                defaults["pauseBreaksDurations"].clone(),
            )
            .expect("valid bundled default"),
            pause_breaks_toggle_shortcut: serde_json::from_value(
                defaults["pauseBreaksToggleShortcut"].clone(),
            )
            .expect("valid bundled default"),
            pause_breaks_until_morning_shortcut: serde_json::from_value(
                defaults["pauseBreaksUntilMorningShortcut"].clone(),
            )
            .expect("valid bundled default"),
            screen: serde_json::from_value(defaults["screen"].clone())
                .expect("valid bundled default"),
            break_content_screen: serde_json::from_value(defaults["breakContentScreen"].clone())
                .expect("valid bundled default"),
            tray_icon_style: serde_json::from_value(defaults["trayIconStyle"].clone())
                .expect("valid bundled default"),
            current_time_in_breaks: serde_json::from_value(defaults["currentTimeInBreaks"].clone())
                .expect("valid bundled default"),
            current_time_in_breaks_font_size: serde_json::from_value(
                defaults["currentTimeInBreaksFontSize"].clone(),
            )
            .expect("valid bundled default"),
            current_time_in_breaks_format: serde_json::from_value(
                defaults["currentTimeInBreaksFormat"].clone(),
            )
            .expect("valid bundled default"),
            show_tray_icon: serde_json::from_value(defaults["showTrayIcon"].clone())
                .expect("valid bundled default"),
            skip_to_next_scheduled_break_shortcut: serde_json::from_value(
                defaults["skipToNextScheduledBreakShortcut"].clone(),
            )
            .expect("valid bundled default"),
            skip_to_next_mini_break_shortcut: serde_json::from_value(
                defaults["skipToNextMiniBreakShortcut"].clone(),
            )
            .expect("valid bundled default"),
            skip_to_next_long_break_shortcut: serde_json::from_value(
                defaults["skipToNextLongBreakShortcut"].clone(),
            )
            .expect("valid bundled default"),
            reset_breaks_shortcut: serde_json::from_value(defaults["resetBreaksShortcut"].clone())
                .expect("valid bundled default"),
            start_last_break_again_shortcut: serde_json::from_value(
                defaults["startLastBreakAgainShortcut"].clone(),
            )
            .expect("valid bundled default"),
            show_tray_menu_in_strict_mode: serde_json::from_value(
                defaults["showTrayMenuInStrictMode"].clone(),
            )
            .expect("valid bundled default"),
            custom_preferences_message: serde_json::from_value(
                defaults["customPreferencesMessage"].clone(),
            )
            .expect("valid bundled default"),
            disable_app_update_features: serde_json::from_value(
                defaults["disableAppUpdateFeatures"].clone(),
            )
            .expect("valid bundled default"),
            hide_preferences_file_location: serde_json::from_value(
                defaults["hidePreferencesFileLocation"].clone(),
            )
            .expect("valid bundled default"),
            hide_strict_mode_preferences: serde_json::from_value(
                defaults["hideStrictModePreferences"].clone(),
            )
            .expect("valid bundled default"),
            mini_break_manual_finish: serde_json::from_value(
                defaults["miniBreakManualFinish"].clone(),
            )
            .expect("valid bundled default"),
            long_break_manual_finish: serde_json::from_value(
                defaults["longBreakManualFinish"].clone(),
            )
            .expect("valid bundled default"),
            break_health_mode: serde_json::from_value(defaults["breakHealthMode"].clone())
                .expect("valid bundled default"),
            open_at_login: serde_json::from_value(defaults["openAtLogin"].clone())
                .expect("valid bundled default"),
            force_x11_on_wayland: serde_json::from_value(defaults["forceX11OnWayland"].clone())
                .expect("valid bundled default"),
            migrated_open_at_login: serde_json::from_value(
                defaults["_migratedOpenAtLogin"].clone(),
            )
            .expect("valid bundled default"),
        }
    }
}
