//! Pure rules: until morning, postpone and skip, exclusions, file names, tray icon, text, displays, locales.

use breaks::{MorningHour, until_morning};

#[test]
fn until_morning_fixed_hour_sunrise_and_dst() {
    for (input, hour, expected) in [
        ("2021-05-25T05:00:00+02:00[Europe/Amsterdam]", 6, 3_600_000),
        ("2021-05-25T07:00:00+02:00[Europe/Amsterdam]", 6, 82_800_000),
        ("2024-03-30T07:00:00+01:00[Europe/Amsterdam]", 6, 79_200_000),
    ] {
        assert_eq!(
            until_morning(&input.parse().unwrap(), &MorningHour::Hour(hour), 0.0, 0.0),
            Some(expected)
        );
    }
    let sunrise = until_morning(
        &"2018-08-04T05:10:00+02:00[Europe/Amsterdam]"
            .parse()
            .unwrap(),
        &MorningHour::Sunrise("sunrise".into()),
        52.0,
        4.0,
    )
    .unwrap();
    assert!((3_540_000..=3_660_000).contains(&sunrise));
}

#[test]
fn postpone_and_skip_actions() {
    for (strict, postpone, passed, can_postpone, can_skip) in [
        (false, true, 20.0, true, false),
        (false, true, 40.0, false, true),
        (true, false, 40.0, false, false),
        (false, false, 20.0, false, true),
    ] {
        assert_eq!(breaks::can_postpone(postpone, passed, 30.0), can_postpone);
        assert_eq!(breaks::can_skip(strict, postpone, passed, 30.0), can_skip);
    }
}

#[test]
fn process_name_matching_for_exclusion_rules() {
    let processes = ["code.exe", "meeting.exe"];
    for (rule, commands, expected) in [
        ("pause", vec!["meeting"], true),
        ("pause", vec!["game"], false),
        ("resume", vec!["meeting"], false),
        ("resume", vec!["game"], true),
    ] {
        let commands: Vec<String> = commands.into_iter().map(Into::into).collect();
        assert_eq!(
            breaks::exclusion_blocks(rule, &commands, &processes),
            expected
        );
    }
}

#[test]
fn media_file_names_allow_only_supported_extensions() {
    for (name, audio, image) in [
        ("bell.wav", true, false),
        ("photo.JPG", false, true),
        ("../bell.wav", false, false),
        ("C:\\x.png", false, false),
        ("picture.webp", false, true),
    ] {
        assert_eq!(breaks::allowed_audio_filename(name), audio);
        assert_eq!(breaks::allowed_image_filename(name), image);
    }
}

#[test]
fn tray_icon_time_table() {
    for (minutes, expected) in [
        (0, "trayNumber0.ico"),
        (42, "trayNumber42.ico"),
        (99, "trayNumber99.ico"),
        (100, "tray.ico"),
    ] {
        assert_eq!(breaks::tray_icon_for_minutes(minutes), expected);
    }
}

#[test]
fn time_remaining_text() {
    for (milliseconds, expected) in [
        (800, "1 second remaining"),
        (59_001, "59 seconds remaining"),
        (60_001, "1 minute remaining"),
        (118_001, "1 minute 58 seconds remaining"),
        (3_600_000, "1 hour remaining"),
    ] {
        assert_eq!(breaks::format_time_remaining(milliseconds), expected);
    }
}

#[test]
fn target_display_and_centered_placement() {
    use breaks::{Display, Rect, target_display, window_position};
    let displays = [
        Display {
            id: 1,
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        },
        Display {
            id: 2,
            bounds: Rect {
                x: -1280,
                y: 0,
                width: 1280,
                height: 1024,
            },
        },
    ];
    let settings = |screen: &str| breaks::Settings {
        all_screens: false,
        screen: screen.into(),
        ..breaks::Settings::default()
    };
    for (setting, index, expected) in [
        ("primary", -1, 1),
        ("cursor", -1, 2),
        ("0", -1, 1),
        ("9", -1, 2),
    ] {
        assert_eq!(
            target_display(&displays, 1, 2, &settings(setting), index)
                .unwrap()
                .id,
            expected
        );
    }
    assert_eq!(
        window_position(displays[1].bounds, 800, 600, false),
        Rect {
            x: -1040,
            y: 212,
            width: 800,
            height: 600
        }
    );
}

#[test]
fn locale_files_parse_and_supply_backend_keys_with_english_fallback() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let english = std::fs::read_to_string(directory.join("en.json")).unwrap();
    let mut count = 0;
    for file in std::fs::read_dir(directory).unwrap() {
        let file = file.unwrap();
        if file
            .path()
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let content = std::fs::read_to_string(file.path()).unwrap();
            assert!(
                breaks::locale_has_backend_keys(&content, &english),
                "{}",
                file.path().display()
            );
            count += 1;
        }
    }
    assert!(count > 1);
}
