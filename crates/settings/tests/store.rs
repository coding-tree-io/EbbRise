//! Saving the config file and importing Stretchly's on first start.

use std::fs;
use std::path::PathBuf;

use settings::Settings;

fn empty_folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("stretch-core-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn saved_settings_load_back_unchanged_with_unknown_keys() {
    let folder = empty_folder("round-trip");
    let own = folder.join("config.json");
    let settings =
        Settings::from_json(r#"{"miniBreakInterval":45000,"keyFromNewerVersion":[1,2]}"#).unwrap();

    settings.save(&own).unwrap();
    let loaded = Settings::load_or_import(&own, &folder.join("absent.json")).unwrap();

    assert_eq!(loaded, settings);
    assert_eq!(loaded.mini_break_interval, 45_000);
    assert_eq!(
        serde_json::to_value(&loaded).unwrap()["keyFromNewerVersion"],
        serde_json::json!([1, 2])
    );
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn first_start_imports_stretchly_config_and_saves_it() {
    let folder = empty_folder("import");
    let stretchly = folder.join("Stretchly").join("config.json");
    let own = folder.join("stretch-core").join("config.json");
    fs::create_dir_all(stretchly.parent().unwrap()).unwrap();
    fs::write(&stretchly, r#"{"miniBreakInterval":45000}"#).unwrap();

    let imported = Settings::load_or_import(&own, &stretchly).unwrap();

    assert_eq!(imported.mini_break_interval, 45_000);
    assert_eq!(
        Settings::from_json(&fs::read_to_string(&own).unwrap()).unwrap(),
        imported
    );
    assert_eq!(
        fs::read_to_string(&stretchly).unwrap(),
        r#"{"miniBreakInterval":45000}"#
    );
    assert_eq!(fs::read_dir(own.parent().unwrap()).unwrap().count(), 1);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn broken_config_is_reported_and_left_untouched() {
    let folder = empty_folder("broken");
    let own = folder.join("config.json");
    fs::write(&own, "{oops").unwrap();

    assert!(Settings::load_or_import(&own, &folder.join("absent.json")).is_err());
    assert_eq!(fs::read_to_string(&own).unwrap(), "{oops");
    fs::remove_dir_all(folder).unwrap();
}
