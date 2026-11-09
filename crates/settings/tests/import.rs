//! Reading Stretchly config files.

use settings::Settings;

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
