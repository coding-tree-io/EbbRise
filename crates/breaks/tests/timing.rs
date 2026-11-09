//! Break timing scenarios on the planner with a fake clock; expected times follow upstream `breaksPlanner.js`.

use breaks::{BreaksPlanner, Event, Settings};

#[test]
fn long_breaks_keep_their_own_clock() {
    let settings = Settings {
        mini_break_interval: 100,
        long_break_every: 300,
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
        // Due at 300, before the middle of the wait after the 320 Mini break: takes its place.
        (320, Event::StartLongBreak),
        // Both clocks restart at 330: Mini at 430, Long due at 630.
        (430, Event::StartMiniBreak),
        (540, Event::StartMiniBreak),
        (650, Event::StartLongBreak),
    ] {
        assert_eq!(planner.advance(start), vec![expected]);
        planner.advance(start + 10);
    }
}

#[test]
fn enabled_break_types_control_the_schedule() {
    let base = Settings {
        mini_break_interval: 100,
        long_break_every: 300,
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
fn long_breaks_alone_come_every_long_break_every() {
    let settings = Settings {
        mini_break: false,
        long_break_every: 2_700_000,
        long_break_duration: 180_000,
        long_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    assert_eq!(planner.advance(2_700_000), vec![Event::StartLongBreak]);
    assert_eq!(planner.advance(2_880_000), vec![Event::FinishLongBreak]);
    assert_eq!(planner.deadline(), Some(5_580_000));
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
fn skip_to_long_restarts_the_long_clock() {
    let settings = Settings {
        mini_break_interval: 100,
        long_break_every: 300,
        mini_break_duration: 10,
        long_break_duration: 10,
        mini_break_notification: false,
        long_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.skip_to_long(0, None);
    assert!(planner.advance(99).is_empty());
    assert_eq!(planner.advance(100), vec![Event::StartLongBreak]);
    planner.advance(110);
    assert_eq!(planner.advance(210), vec![Event::StartMiniBreak]);
    planner.advance(220);
    planner.skip_to_mini(220, None);
    assert_eq!(planner.advance(320), vec![Event::StartMiniBreak]);
    planner.advance(330);
    // Long due at 110 + 300 = 410: takes the place of the 430 Mini break.
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
fn notification_comes_first_and_counts_in_tray_time_and_progress() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification_interval: 20,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    assert_eq!(planner.time_to_next_break(50), Some(50));
    assert_eq!(planner.progress_percentage(50), 50);
    assert!(planner.advance(79).is_empty());
    assert_eq!(planner.advance(80), vec![Event::MiniBreakNotification]);
    assert_eq!(planner.time_to_next_break(90), Some(10));
    assert_eq!(planner.progress_percentage(90), 50);
    assert_eq!(planner.advance(100), vec![Event::StartMiniBreak]);
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
}

#[test]
fn pause_ending_during_dnd_waits_for_dnd_to_end() {
    let settings = Settings {
        mini_break_interval: 100,
        mini_break_notification: false,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings, 0);
    planner.pause(10, 200);
    planner.focus_assist(20, 1);
    assert_eq!(planner.advance(210), vec![Event::ResumeBreaks]);
    assert!(!planner.paused());
    assert!(planner.advance(1000).is_empty());
    planner.focus_assist(1000, 0);
    assert_eq!(planner.advance(1100), vec![Event::StartMiniBreak]);
}

#[test]
fn focus_assist_ignored_when_unsupported_or_not_monitored() {
    for (monitor_dnd, value) in [(true, -1), (false, 1)] {
        let settings = Settings {
            mini_break_interval: 100,
            mini_break_notification: false,
            monitor_dnd,
            ..Settings::default()
        };
        let mut planner = BreaksPlanner::new(settings, 0);
        planner.focus_assist(50, value);
        assert_eq!(planner.advance(100), vec![Event::StartMiniBreak]);
    }
}

#[test]
fn idle_past_reset_time_stops_breaks_until_return() {
    let settings = Settings {
        mini_break_interval: 100_000,
        mini_break_notification: false,
        natural_breaks_inactivity_reset_time: 30_000,
        ..Settings::default()
    };
    let mut planner = BreaksPlanner::new(settings.clone(), 0);
    planner.idle(30_001, 30_001);
    assert!(planner.advance(100_000).is_empty());
    planner.idle(100_001, 0);
    assert_eq!(planner.advance(200_001), vec![Event::StartMiniBreak]);
    // DND ending while the user is away must not lose the return (upstream resets on both).
    let mut dnd_then_away = BreaksPlanner::new(settings, 0);
    dnd_then_away.focus_assist(10, 1);
    dnd_then_away.idle(30_001, 30_001);
    dnd_then_away.focus_assist(40_000, 0);
    assert!(dnd_then_away.advance(100_000).is_empty());
    dnd_then_away.idle(100_001, 0);
    assert_eq!(dnd_then_away.advance(200_001), vec![Event::StartMiniBreak]);
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

fn exclusion(rule: &str) -> Settings {
    Settings {
        mini_break_interval: 100,
        mini_break_duration: 50,
        mini_break_notification: false,
        app_exclusions: vec![breaks::AppExclusion {
            rule: rule.into(),
            active: true,
            commands: vec!["meeting".into()],
        }],
        ..Settings::default()
    }
}

#[test]
fn pause_exclusion_finishes_break_and_resumes_when_removed() {
    let mut planner = BreaksPlanner::new(exclusion("pause"), 0);
    planner.advance(100);
    assert_eq!(
        planner.processes_changed(110, &["meeting.exe"]),
        vec![Event::FinishMiniBreak]
    );
    assert!(planner.advance(300).is_empty());
    planner.processes_changed(300, &[]);
    assert_eq!(planner.advance(400), vec![Event::StartMiniBreak]);
    // An exclusion that ends during a break leaves the break running (upstream guard).
    planner.processes_changed(410, &["meeting.exe"]);
    planner.skip_to_mini(410, None);
    assert_eq!(planner.advance(510), vec![Event::StartMiniBreak]);
    planner.processes_changed(520, &[]);
    assert_eq!(planner.advance(560), vec![Event::FinishMiniBreak]);
}

#[test]
fn resume_exclusion_runs_breaks_only_while_its_app_runs() {
    let mut planner = BreaksPlanner::new(exclusion("resume"), 0);
    planner.processes_changed(10, &[]);
    assert!(planner.advance(200).is_empty());
    planner.processes_changed(200, &["meeting.exe"]);
    assert_eq!(planner.advance(300), vec![Event::StartMiniBreak]);
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
