//! Reading Stretchly config files.

use settings::Settings;

#[test]
fn media_and_full_screen_settings_default_and_round_trip() {
    let defaults = Settings::default();
    assert!(defaults.no_breaks_in_full_screen);
    assert!(!defaults.pause_media_during_breaks);
    let settings =
        Settings::from_json(r#"{"noBreaksInFullScreen":false,"pauseMediaDuringBreaks":true}"#)
            .unwrap();
    let json = serde_json::to_value(settings).unwrap();
    assert_eq!(json["noBreaksInFullScreen"], false);
    assert_eq!(json["pauseMediaDuringBreaks"], true);
}

#[test]
fn stretchly_config_keeps_defaults_for_missing_keys() {
    let settings = Settings::from_json(
        r#"{"miniBreakInterval":45000,"morningHour":"sunrise","miniBreak":false}"#,
    )
    .unwrap();
    assert_eq!(settings.mini_break_interval, 45_000);
    assert!(!settings.mini_break);
    assert_eq!(settings.long_break_duration, 300_000);
    assert!(!settings.mini_break_ideas.is_empty());
    assert_eq!(
        serde_json::to_value(&settings).unwrap()["morningHour"],
        "sunrise"
    );
    assert!(Settings::from_json("[]").is_err());
}

#[test]
fn long_break_every_comes_from_the_stretchly_count_until_it_is_set() {
    let stretchly =
        Settings::from_json(r#"{"miniBreakInterval":1200000,"longBreakInterval":3}"#).unwrap();
    assert_eq!(stretchly.long_break_every, 4_800_000);
    let own = Settings::from_json(r#"{"longBreakEvery":2700000,"longBreakInterval":3}"#).unwrap();
    assert_eq!(own.long_break_every, 2_700_000);
    assert_eq!(Settings::default().long_break_every, 1_800_000);
}
