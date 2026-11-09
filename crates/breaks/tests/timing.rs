use breaks::{BreaksPlanner, Event, Settings};

#[test]
fn mini_and_long_break_order() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_duration: 10,
        long_break_duration: 10,
        mini_break_notification: false,
        long_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    for (start, expected) in [
        (100, Event::StartMiniBreak),
        (210, Event::StartMiniBreak),
        (320, Event::StartLongBreak),
    ] {
        assert_eq!(planner.advance(start), vec![expected]);
        planner.advance(start + 10);
    }
}

#[test]
fn enabled_break_types_control_the_schedule() {
    let base = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        long_break_notification: false,
        ..Settings::default()
    };
    let mut mini = BreaksPlanner::new(
        Settings {
            long_break: false,
            ..base.clone()
        },
        0,
    );
    assert_eq!(mini.advance(100), vec![Event::StartMiniBreak]);
    let mut long = BreaksPlanner::new(
        Settings {
            mini_break: false,
            ..base.clone()
        },
        0,
    );
    assert!(long.advance(299).is_empty());
    assert_eq!(long.advance(300), vec![Event::StartLongBreak]);
    let mut neither = BreaksPlanner::new(
        Settings {
            mini_break: false,
            long_break: false,
            ..base
        },
        0,
    );
    assert!(neither.advance(1000).is_empty());
}

#[test]
fn notification_precedes_break_by_configured_interval() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification_interval: 20,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    assert!(planner.advance(79).is_empty());
    assert_eq!(planner.advance(80), vec![Event::MiniBreakNotification]);
    assert!(planner.advance(99).is_empty());
    assert_eq!(planner.advance(100), vec![Event::StartMiniBreak]);
}

#[test]
fn postpone_keeps_notification_and_enforces_limit() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification_interval: 20,
        mini_break_postpone_time: 50,
        mini_break_postpones_limit: 1,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.advance(80);
    planner.advance(100);
    assert!(planner.postpone(100));
    assert_eq!(planner.advance(130), vec![Event::MiniBreakNotification]);
    assert_eq!(planner.advance(150), vec![Event::StartMiniBreak]);
    assert!(!planner.postpone(150));
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_duration: 100,
        mini_break_notification: false,
        mini_break_postponable_duration_percent: 30,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.advance(100);
    assert!(!planner.postpone(131));
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        mini_break_postpone: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.advance(100);
    assert!(!planner.postpone(100));
}

#[test]
fn skip_to_break_preserves_following_order() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_duration: 10,
        long_break_duration: 10,
        mini_break_notification: false,
        long_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.skip_to_long(0);
    assert!(planner.advance(99).is_empty());
    assert_eq!(planner.advance(100), vec![Event::StartLongBreak]);
    planner.advance(110);
    assert_eq!(planner.advance(210), vec![Event::StartMiniBreak]);
    planner.advance(220);
    planner.skip_to_mini(220);
    assert_eq!(planner.advance(320), vec![Event::StartMiniBreak]);
    planner.advance(330);
    assert_eq!(planner.advance(430), vec![Event::StartLongBreak]);
}

#[test]
fn timed_and_indefinite_pause_resume_and_reset() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.pause(10, 200);
    assert!(planner.advance(209).is_empty());
    assert_eq!(planner.advance(210), vec![Event::ResumeBreaks]);
    assert_eq!(planner.advance(310), vec![Event::StartMiniBreak]);
    planner.pause(320, 1);
    assert!(planner.advance(1000).is_empty());
    planner.resume(1000);
    assert_eq!(planner.advance(1100), vec![Event::StartMiniBreak]);
    planner.reset(1110);
    assert_eq!(planner.advance(1210), vec![Event::StartMiniBreak]);
}

#[test]
fn pause_until_morning_resumes_at_morning_then_starts_a_new_cycle() {
    let morning = breaks::until_morning(
        &"2024-05-25T05:00:00+02:00[Europe/Amsterdam]"
            .parse()
            .unwrap(),
        &breaks::MorningHour::Hour(6),
        0.0,
        0.0,
    )
    .unwrap();
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.pause(0, morning);
    assert!(planner.advance(morning - 1).is_empty());
    assert_eq!(planner.advance(morning), vec![Event::ResumeBreaks]);
    assert_eq!(planner.advance(morning + 100), vec![Event::StartMiniBreak]);
}

#[test]
fn manual_finish_waits_and_then_schedules_next_break() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_duration: 10,
        mini_break_notification: false,
        mini_break_manual_finish: true,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    assert_eq!(planner.advance(100), vec![Event::StartMiniBreak]);
    assert_eq!(planner.advance(110), vec![Event::AwaitMiniBreakFinish]);
    assert!(planner.advance(1000).is_empty());
    assert_eq!(planner.finish_break(1000), Some(Event::FinishMiniBreak));
    assert_eq!(planner.advance(1100), vec![Event::StartMiniBreak]);
}

#[test]
fn tray_time_and_progress_include_notification_interval() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification_interval: 20,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    assert_eq!(planner.time_to_next_break(50), Some(50));
    assert_eq!(planner.progress_percentage(50), 50);
    planner.advance(80);
    assert_eq!(planner.time_to_next_break(90), Some(10));
    assert_eq!(planner.progress_percentage(90), 50);
    planner.advance(100);
    assert_eq!(planner.time_to_next_break(101), None);
}

#[test]
fn dnd_state_changes_respect_active_break_and_pause() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_duration: 10,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    assert_eq!(planner.advance(100), vec![Event::StartMiniBreak]);
    planner.focus_assist(101, 1);
    assert_eq!(planner.advance(110), vec![Event::FinishMiniBreak]);
    assert!(planner.advance(300).is_empty());
    planner.focus_assist(300, 0);
    assert_eq!(planner.advance(400), vec![Event::StartMiniBreak]);
    planner.pause(401, 1);
    planner.focus_assist(402, 1);
    planner.focus_assist(403, 0);
    assert!(planner.advance(1000).is_empty());
    let settings = Settings {
        mini_break_interval: 100_000,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut event_driven = BreaksPlanner::new(settings.clone(), 0);
    let mut polled = BreaksPlanner::new(settings, 0);
    event_driven.focus_assist(10, 1);
    polled.focus_assist(2_000, 1);
    assert!(event_driven.advance(100_000).is_empty());
    assert!(polled.advance(100_000).is_empty());
    event_driven.focus_assist(100_010, 0);
    polled.focus_assist(102_000, 0);
    assert_eq!(event_driven.advance(200_010), vec![Event::StartMiniBreak]);
    assert_eq!(polled.advance(202_000), vec![Event::StartMiniBreak]);
}

#[test]
fn resume_does_not_start_breaks_while_dnd_is_active() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.pause(10, 1);
    planner.focus_assist(20, 1);
    planner.resume(30);
    assert!(planner.advance(200).is_empty());
    planner.focus_assist(200, 0);
    assert!(planner.advance(300).is_empty());
    planner.resume(300);
    assert_eq!(planner.advance(400), vec![Event::StartMiniBreak]);
}

#[test]
fn unsupported_focus_assist_does_not_pause() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.focus_assist(50, -1);
    assert_eq!(planner.advance(100), vec![Event::StartMiniBreak]);
}

#[test]
fn disabled_dnd_monitor_ignores_focus_assist_events() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        monitor_dnd: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.focus_assist(50, 1);
    assert_eq!(planner.advance(100), vec![Event::StartMiniBreak]);
}

#[test]
fn idle_reset_and_return_match_polling_within_one_interval() {
    let settings = Settings {
        mini_break_interval: 100_000,
        mini_break_notification: false,
        natural_breaks_inactivity_reset_time: 30_000,
        ..Settings::default()
    };
    let mut event_driven = BreaksPlanner::new(settings.clone(), 0);
    let mut polled = BreaksPlanner::new(settings, 0);
    event_driven.idle(30_001, 30_001);
    polled.idle(32_000, 32_000);
    assert!(event_driven.advance(100_000).is_empty());
    assert!(polled.advance(100_000).is_empty());
    event_driven.idle(100_001, 0);
    polled.idle(102_000, 0);
    assert_eq!(event_driven.advance(200_001), vec![Event::StartMiniBreak]);
    assert_eq!(polled.advance(202_000), vec![Event::StartMiniBreak]);
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_duration: 50_000,
        mini_break_notification: false,
        natural_breaks_inactivity_reset_time: 30_000,
        ..Settings::default()
    };
    let mut during_break = BreaksPlanner::new(settings, 0);
    during_break.advance(100);
    during_break.idle(30_101, 30_001);
    assert_eq!(during_break.advance(50_100), vec![Event::FinishMiniBreak]);
    assert!(during_break.advance(50_200).is_empty());
    during_break.idle(50_201, 0);
    assert_eq!(during_break.advance(50_301), vec![Event::StartMiniBreak]);
}

#[test]
fn pause_exclusion_finishes_break_and_resumes_when_removed() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_duration: 50,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.advance(100);
    assert_eq!(
        planner.app_exclusion(110, true),
        vec![Event::FinishMiniBreak]
    );
    assert!(planner.advance(300).is_empty());
    planner.app_exclusion(300, false);
    assert_eq!(planner.advance(400), vec![Event::StartMiniBreak]);
}

#[test]
fn resume_exclusion_event_stops_and_restarts_schedule() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.resume_exclusion(10, false);
    assert!(planner.advance(200).is_empty());
    planner.resume_exclusion(200, true);
    assert_eq!(planner.advance(300), vec![Event::StartMiniBreak]);
}

#[test]
fn process_events_and_polled_processes_produce_same_outcome() {
    let settings = Settings {
        mini_break_interval: 100_000,
        mini_break_notification: false,
        app_exclusions: vec![breaks::AppExclusion {
            rule: "pause".into(),
            active: true,
            commands: vec!["meeting".into()],
        }],
        ..Settings::default()
    };
    let mut event_driven = BreaksPlanner::new(settings.clone(), 0);
    let mut polled = BreaksPlanner::new(settings, 0);
    event_driven.processes_changed(10, &["meeting.exe"]);
    polled.processes_changed(2_000, &["meeting.exe"]);
    assert!(event_driven.advance(100_000).is_empty());
    assert!(polled.advance(100_000).is_empty());
    event_driven.processes_changed(100_010, &[]);
    polled.processes_changed(102_000, &[]);
    assert_eq!(event_driven.advance(200_010), vec![Event::StartMiniBreak]);
    assert_eq!(polled.advance(202_000), vec![Event::StartMiniBreak]);
}

#[test]
fn suspend_resumes_only_system_owned_pause() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.suspend(50);
    assert!(planner.advance(500).is_empty());
    planner.wake(500);
    assert_eq!(planner.advance(600), vec![Event::StartMiniBreak]);
    planner.pause(601, 1);
    planner.suspend(602);
    planner.wake(700);
    assert!(planner.advance(1000).is_empty());
}

#[test]
fn deadline_during_sleep_fires_once_if_suspend_pause_disabled() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        pause_for_suspend_or_lock: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.suspend(40);
    planner.wake(200);
    assert_eq!(planner.advance(200), vec![Event::StartMiniBreak]);
    assert!(planner.advance(200).is_empty());
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        pause_for_suspend_or_lock: false,
        ..Settings::default()
    };
    let mut before_deadline = BreaksPlanner::new(settings, 0);
    before_deadline.suspend(20);
    before_deadline.wake(50);
    assert!(before_deadline.advance(99).is_empty());
    assert_eq!(before_deadline.advance(100), vec![Event::StartMiniBreak]);
}

#[test]
fn settings_import_stretchly_json_with_defaults() {
    assert!(Settings::windows_defaults(true, false).disable_app_update_features);
    assert!(Settings::windows_defaults(false, true).disable_app_update_features);
    let settings: Settings = serde_json::from_str(
        r#"{"miniBreakInterval":45000,"morningHour":"sunrise","miniBreak":false}"#,
    )
    .unwrap();
    assert_eq!(settings.mini_break_interval, 45_000);
    assert!(!settings.mini_break);
    assert_eq!(settings.long_break_duration, 300_000);
    assert_eq!(
        serde_json::to_value(&settings).unwrap()["morningHour"],
        "sunrise"
    );
    assert!(!settings.mini_break_ideas.is_empty());
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../settings/src/default-settings.json")).unwrap();
    let actual = serde_json::to_value(Settings::default()).unwrap();
    assert_eq!(
        actual.as_object().unwrap().len(),
        expected.as_object().unwrap().len()
    );
    for (key, value) in expected.as_object().unwrap() {
        if value.is_number() {
            assert_eq!(actual[key].as_f64(), value.as_f64(), "{key}");
        } else {
            assert_eq!(&actual[key], value, "{key}");
        }
    }
}
