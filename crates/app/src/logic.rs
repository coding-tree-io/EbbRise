//! The app's behaviour without Windows: signals and menu choices go in with the time,
//! toasts and the next wake-up come out. `main.rs` only connects this to the OS.

mod command;

use breaks::{BreaksPlanner, Event, format_time_remaining};
use platform::Signal;
use settings::Settings;

/// How often to check for the user's return while they are away (no input event exists for that).
const AWAY_CHECK_MS: i64 = 2_000;
pub const HOUR_MS: i64 = 3_600_000;
const FOCUS_ASSIST_UNSUPPORTED: i32 = -1;

/// Tray menu choices. `id` is the menu item id, so a typo cannot silently disable an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Resume,
    PauseHour,
    PauseUntilMorning,
    PauseIndefinitely,
    SkipToMini,
    SkipToLong,
    Reset,
    Quit,
}

impl MenuAction {
    const ALL: [Self; 8] = [
        Self::Resume,
        Self::PauseHour,
        Self::PauseUntilMorning,
        Self::PauseIndefinitely,
        Self::SkipToMini,
        Self::SkipToLong,
        Self::Reset,
        Self::Quit,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Resume => "resume",
            Self::PauseHour => "pause-hour",
            Self::PauseUntilMorning => "pause-morning",
            Self::PauseIndefinitely => "pause-forever",
            Self::SkipToMini => "skip-mini",
            Self::SkipToLong => "skip-long",
            Self::Reset => "reset",
            Self::Quit => "quit",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub title: String,
    pub text: String,
}

/// The next timer: fire after `delay` ms; Windows may shift it by `tolerance` ms to batch wake-ups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wake {
    pub delay: i64,
    pub tolerance: u32,
}

/// What the core asks the OS for only when it needs it.
pub trait Environment {
    /// Executable names of the running processes.
    fn running_processes(&self) -> Vec<String>;
    /// Milliseconds until the configured morning; `None` when there is no morning time.
    fn until_morning(&self, settings: &Settings) -> Option<i64>;
}

/// The words a break shows: a Mini break shows only `text`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BreakIdea {
    title: String,
    text: String,
}

/// The kind and words of the last break, for the `last` command.
#[derive(Clone, Debug)]
struct LastBreak {
    long: bool,
    idea: BreakIdea,
}

pub struct Core {
    planner: BreaksPlanner,
    exclusion_checked_at: i64,
    /// Words that a `mini` or `long` command set for the next break (upstream `nextIdea`).
    next_idea: Option<command::IdeaOverride>,
    last_break: Option<LastBreak>,
}

impl Core {
    /// `focus_assist` is the value at start, `None` when Windows cannot report it.
    pub fn new(settings: Settings, now: i64, focus_assist: Option<i32>) -> Self {
        let mut planner = BreaksPlanner::new(settings, now);
        planner.focus_assist(now, focus_assist.unwrap_or(FOCUS_ASSIST_UNSUPPORTED));
        Self {
            planner,
            exclusion_checked_at: now,
            next_idea: None,
            last_break: None,
        }
    }

    pub fn settings(&self) -> &Settings {
        self.planner.settings()
    }

    /// `now` is awake time and `idle` is time since the last input, both in ms.
    pub fn on_signal(
        &mut self,
        signal: Signal,
        now: i64,
        idle: i64,
        os: &impl Environment,
    ) -> Vec<Toast> {
        match signal {
            Signal::Timer => {
                self.planner.idle(now, idle);
                let mut toasts = self.check_app_exclusion(now, os);
                toasts.extend(self.run_due_steps(now));
                return toasts;
            }
            Signal::Suspend | Signal::Lock => self.planner.suspend(now),
            Signal::Resume | Signal::Unlock => self.planner.wake(now),
            Signal::FocusAssist(value) => self.planner.focus_assist(now, value),
            Signal::Command(args) => self.on_command(&args, now, os),
        }
        Vec::new()
    }

    /// When the next app exclusion check is due; `None` without an active rule.
    fn app_exclusion_check_due(&self) -> Option<i64> {
        let settings = self.settings();
        let active_rule = settings.app_exclusions.iter().any(|rule| rule.active);
        active_rule.then(|| self.exclusion_checked_at + settings.app_exclusions_check_interval)
    }

    fn check_app_exclusion(&mut self, now: i64, os: &impl Environment) -> Vec<Toast> {
        if !self.app_exclusion_check_due().is_some_and(|due| due <= now) {
            return Vec::new();
        }
        self.exclusion_checked_at = now;
        let names = os.running_processes();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let events = self.planner.processes_changed(now, &names);
        events
            .into_iter()
            .filter_map(|event| self.carry_out(event, now))
            .collect()
    }

    /// Several steps can be due at once, for example after a long busy period.
    fn run_due_steps(&mut self, now: i64) -> Vec<Toast> {
        let mut toasts = Vec::new();
        while self
            .planner
            .deadline()
            .is_some_and(|deadline| deadline <= now)
        {
            for event in self.planner.advance(now) {
                toasts.extend(self.carry_out(event, now));
            }
        }
        toasts
    }

    fn carry_out(&mut self, event: Event, now: i64) -> Option<Toast> {
        let event = match event {
            // shortcut: no break window yet to press "Finish"; finish at once. Wire to the UI when it exists.
            Event::AwaitMiniBreakFinish | Event::AwaitLongBreakFinish => {
                self.planner.finish_break(now)?
            }
            event => event,
        };
        let idea = match event {
            Event::StartMiniBreak => self.take_idea(false, now),
            Event::StartLongBreak => self.take_idea(true, now),
            _ => BreakIdea::default(),
        };
        toast(self.settings(), &event, idea)
    }

    /// The words for a starting break: the `mini` or `long` command's words, else an enabled idea.
    fn take_idea(&mut self, long: bool, now: i64) -> BreakIdea {
        let settings = self.settings();
        let picked = if long {
            pick(&settings.long_break_ideas, now).map(|idea| BreakIdea {
                title: idea.data.first().cloned().unwrap_or_default(),
                text: idea.data.get(1).cloned().unwrap_or_default(),
            })
        } else {
            pick(&settings.mini_break_ideas, now).map(|idea| BreakIdea {
                title: String::new(),
                text: idea.data.clone(),
            })
        };
        let picked = picked.unwrap_or_default();
        let idea = match self.next_idea.take() {
            Some(words) => BreakIdea {
                title: words.title.unwrap_or(picked.title),
                text: words.text.unwrap_or(picked.text),
            },
            None => picked,
        };
        self.last_break = Some(LastBreak {
            long,
            idea: idea.clone(),
        });
        idea
    }

    /// Pause until morning pauses indefinitely when there is no morning time.
    pub fn on_menu(&mut self, action: MenuAction, now: i64, os: &impl Environment) {
        match action {
            MenuAction::Resume if self.planner.paused() => self.planner.resume(now),
            MenuAction::PauseHour => self.planner.pause(now, HOUR_MS),
            MenuAction::PauseUntilMorning => {
                let duration = os
                    .until_morning(self.settings())
                    .unwrap_or(BreaksPlanner::INDEFINITELY);
                self.planner.pause(now, duration);
            }
            MenuAction::PauseIndefinitely => self.planner.pause(now, BreaksPlanner::INDEFINITELY),
            MenuAction::SkipToMini => self.planner.skip_to_mini(now, None),
            MenuAction::SkipToLong => self.planner.skip_to_long(now, None),
            MenuAction::Reset => self.planner.reset(now),
            MenuAction::Resume | MenuAction::Quit => {}
        }
    }

    /// The one timer the app needs: the earliest of the break deadline, the idle check and
    /// the app exclusion check. On a tie the break deadline wins, with its tighter tolerance.
    pub fn next_wake(&self, now: i64, idle: i64) -> Option<Wake> {
        let settings = self.settings();
        let break_due = self.planner.deadline().map(|deadline| Wake {
            delay: deadline - now,
            tolerance: 10,
        });
        let idle_check = settings.natural_breaks.then(|| {
            let delay = if self.planner.away() {
                AWAY_CHECK_MS
            } else {
                let earliest_reset_crossing = settings.natural_breaks_inactivity_reset_time - idle;
                earliest_reset_crossing.max(1_000)
            };
            Wake {
                delay,
                tolerance: 1_000,
            }
        });
        let exclusion_check = self.app_exclusion_check_due().map(|due| Wake {
            delay: (due - now).max(0),
            tolerance: 1_000,
        });
        [break_due, idle_check, exclusion_check]
            .into_iter()
            .flatten()
            .min_by_key(|wake| (wake.delay, wake.tolerance))
    }

    /// Tray tooltip and whether to show the paused icon.
    pub fn tray(&self, now: i64) -> (String, bool) {
        let paused = self.planner.paused();
        let text = if paused {
            match self.planner.deadline() {
                Some(deadline) => format!("Paused, resumes in {}", duration(deadline - now)),
                None => "Paused".into(),
            }
        } else if self.planner.in_break() {
            "Break in progress".into()
        } else {
            match self.planner.time_to_next_break(now) {
                Some(remaining) => format!("Next break in {}", duration(remaining)),
                None => "Waiting (Do Not Disturb or away)".into(),
            }
        };
        (text, paused)
    }
}

fn toast(s: &Settings, event: &Event, idea: BreakIdea) -> Option<Toast> {
    let (title, text) = match event {
        Event::MiniBreakNotification => (
            "Mini break soon".into(),
            format!(
                "Mini break in {}",
                duration(s.mini_break_notification_interval)
            ),
        ),
        Event::LongBreakNotification => (
            "Long break soon".into(),
            format!(
                "Long break in {}",
                duration(s.long_break_notification_interval)
            ),
        ),
        Event::StartMiniBreak => (
            format!("Mini break: {}", duration(s.mini_break_duration)),
            idea.text,
        ),
        Event::StartLongBreak => (
            format!("Long break: {}", duration(s.long_break_duration)),
            [idea.title, idea.text]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(": "),
        ),
        Event::FinishMiniBreak | Event::FinishLongBreak => {
            ("Break over".into(), "Back to work.".into())
        }
        Event::AwaitMiniBreakFinish | Event::AwaitLongBreakFinish | Event::ResumeBreaks => {
            return None;
        }
    };
    Some(Toast { title, text })
}

/// "9 minutes 58 seconds" from the shared time-remaining text.
fn duration(milliseconds: i64) -> String {
    let text = format_time_remaining(milliseconds);
    text.strip_suffix(" remaining").unwrap_or(&text).to_owned()
}

/// Picks an enabled idea; the clock is random enough for this.
fn pick<T>(ideas: &[settings::Idea<T>], now: i64) -> Option<&settings::Idea<T>> {
    let enabled: Vec<_> = ideas.iter().filter(|idea| idea.enabled).collect();
    (!enabled.is_empty()).then(|| enabled[now as usize % enabled.len()])
}

#[cfg(test)]
mod tests {
    //! Discrete-event simulation: virtual time jumps straight to each wake-up the core asks for,
    //! so hours of app time run in microseconds. Expected times follow upstream breaksPlanner.js.

    use super::*;

    const USER_TIMER_MINIMUM_MS: i64 = 10;

    /// Short demo timings: mini every 30 s (notice 10 s before), 10 s long; long break every third.
    const DEMO: &str = r#""miniBreakInterval":30000,"miniBreakDuration":10000,
        "miniBreakNotificationInterval":10000,"longBreakInterval":2,"longBreakDuration":15000,
        "longBreakNotificationInterval":10000,"naturalBreaks":true,
        "naturalBreaksInactivityResetTime":300000,"monitorDnd":true"#;

    struct Sim {
        core: Core,
        now: i64,
        /// Start of the user's current absence; `None` while they use the PC.
        away_since: Option<i64>,
        /// (time, toast title) for each toast.
        log: Vec<(i64, String)>,
        wakes: usize,
        os: FakeOs,
        /// Text of the last toast.
        last_text: String,
    }

    #[derive(Default)]
    struct FakeOs {
        /// Process names Windows would list now.
        running: Vec<String>,
        process_reads: std::cell::Cell<usize>,
        morning: Option<i64>,
    }

    impl Environment for FakeOs {
        fn running_processes(&self) -> Vec<String> {
            self.process_reads.set(self.process_reads.get() + 1);
            self.running.clone()
        }

        fn until_morning(&self, _: &Settings) -> Option<i64> {
            self.morning
        }
    }

    impl Sim {
        fn new(extra_json: &str) -> Self {
            // Later keys win, so a test can override a demo timing.
            let settings = Settings::from_json(&format!("{{{DEMO}{extra_json}}}")).unwrap();
            Self {
                core: Core::new(settings, 0, Some(0)),
                now: 0,
                away_since: None,
                log: Vec::new(),
                wakes: 0,
                os: FakeOs::default(),
                last_text: String::new(),
            }
        }

        fn idle(&self) -> i64 {
            self.away_since.map_or(0, |since| self.now - since)
        }

        fn run_until(&mut self, until: i64) {
            while let Some(at) = self
                .core
                .next_wake(self.now, self.idle())
                .map(|wake| self.now + wake.delay.max(USER_TIMER_MINIMUM_MS))
                .filter(|at| *at <= until)
            {
                self.now = at;
                self.wakes += 1;
                self.signal(Signal::Timer);
            }
            self.now = until;
        }

        fn signal(&mut self, signal: Signal) {
            let toasts = self.core.on_signal(signal, self.now, self.idle(), &self.os);
            for toast in toasts {
                self.log.push((self.now, toast.title));
                self.last_text = toast.text;
            }
        }

        fn command(&mut self, args: &[&str]) {
            self.signal(Signal::Command(
                args.iter().map(|arg| arg.to_string()).collect(),
            ));
        }

        fn take_log(&mut self) -> Vec<(i64, String)> {
            std::mem::take(&mut self.log)
        }
    }

    fn log(entries: &[(i64, &str)]) -> Vec<(i64, String)> {
        entries
            .iter()
            .map(|(at, title)| (*at, title.to_string()))
            .collect()
    }

    #[test]
    fn full_cycle_follows_upstream_timings() {
        let mut sim = Sim::new("");
        sim.run_until(130_000);
        assert_eq!(
            sim.take_log(),
            log(&[
                (20_000, "Mini break soon"),
                (30_000, "Mini break: 10 seconds"),
                (40_000, "Break over"),
                (60_000, "Mini break soon"),
                (70_000, "Mini break: 10 seconds"),
                (80_000, "Break over"),
                (100_000, "Long break soon"),
                (110_000, "Long break: 15 seconds"),
                (125_000, "Break over"),
            ])
        );
        // One wake-up per step: no polling while the user works.
        assert_eq!(sim.wakes, 9);
        assert_eq!(
            sim.os.process_reads.get(),
            0,
            "no active exclusion rule, no process list"
        );
    }

    #[test]
    fn pause_for_an_hour_then_breaks_restart() {
        let mut sim = Sim::new("");
        sim.run_until(5_000);
        sim.core.on_menu(MenuAction::PauseHour, sim.now, &sim.os);
        assert_eq!(
            sim.core.tray(sim.now),
            ("Paused, resumes in 1 hour".into(), true)
        );
        sim.run_until(5_000 + HOUR_MS + 25_000);
        assert_eq!(
            sim.take_log(),
            log(&[(5_000 + HOUR_MS + 20_000, "Mini break soon")])
        );
        // Only the 5-minute idle checks woke the app during the hour.
        assert!(sim.wakes <= 14, "{} wake-ups", sim.wakes);
    }

    #[test]
    fn pause_until_morning_uses_the_given_time_and_falls_back_to_indefinite() {
        let mut sim = Sim::new("");
        sim.os.morning = Some(1_000);
        sim.core.on_menu(MenuAction::PauseUntilMorning, 0, &sim.os);
        sim.run_until(1_000);
        assert!(!sim.core.tray(sim.now).1, "resumed after the morning delay");
        sim.os.morning = None;
        sim.core
            .on_menu(MenuAction::PauseUntilMorning, sim.now, &sim.os);
        assert_eq!(sim.core.tray(sim.now), ("Paused".into(), true));
        sim.core.on_menu(MenuAction::Resume, sim.now, &sim.os);
        assert!(!sim.core.tray(sim.now).1);
    }

    #[test]
    fn natural_break_stops_breaks_and_restarts_them_on_return() {
        let mut sim = Sim::new(r#","miniBreakInterval":600000"#);
        sim.away_since = Some(0);
        sim.run_until(400_000);
        assert_eq!(sim.take_log(), log(&[]));
        assert_eq!(sim.core.tray(sim.now).0, "Waiting (Do Not Disturb or away)");

        sim.away_since = None;
        sim.run_until(400_000 + AWAY_CHECK_MS + 600_000);
        let log = sim.take_log();
        let (at, title) = &log[0];
        assert_eq!(title, "Mini break soon");
        let at = *at;
        // The return is seen at the next away check, at most 2 s late.
        assert!(
            (400_000 + 590_000..=400_000 + 590_000 + AWAY_CHECK_MS).contains(&at),
            "{at}"
        );
    }

    #[test]
    fn lock_pauses_when_set_and_unlock_restarts() {
        let mut sim = Sim::new(r#","pauseForSuspendOrLock":true"#);
        sim.run_until(5_000);
        sim.signal(Signal::Lock);
        sim.run_until(100_000);
        assert_eq!(sim.take_log(), log(&[]));
        sim.signal(Signal::Unlock);
        sim.run_until(125_000);
        assert_eq!(sim.take_log(), log(&[(120_000, "Mini break soon")]));
    }

    #[test]
    fn sleep_without_pause_keeps_the_remaining_time() {
        let mut sim = Sim::new(r#","pauseForSuspendOrLock":false"#);
        sim.run_until(5_000);
        // The awake clock stands still while the PC sleeps, so resume arrives at the same `now`.
        sim.signal(Signal::Suspend);
        sim.signal(Signal::Resume);
        sim.run_until(25_000);
        assert_eq!(sim.take_log(), log(&[(20_000, "Mini break soon")]));
    }

    #[test]
    fn do_not_disturb_holds_breaks_until_it_ends() {
        let mut sim = Sim::new("");
        sim.signal(Signal::FocusAssist(2));
        sim.run_until(50_000);
        assert_eq!(sim.take_log(), log(&[]));
        sim.signal(Signal::FocusAssist(0));
        sim.run_until(75_000);
        assert_eq!(sim.take_log(), log(&[(70_000, "Mini break soon")]));
    }

    #[test]
    fn pause_exclusion_holds_breaks_while_its_app_runs() {
        let mut sim =
            Sim::new(r#","appExclusions":[{"rule":"pause","active":true,"commands":["meeting"]}]"#);
        sim.os.running = vec!["meeting.exe".into()];
        sim.run_until(50_000);
        assert_eq!(sim.take_log(), log(&[]));

        sim.os.running.clear();
        sim.run_until(80_000);
        // The next 2 s check, at 52 s, sees the app gone and restarts the cycle.
        assert_eq!(sim.take_log(), log(&[(72_000, "Mini break soon")]));
        assert_eq!(sim.os.process_reads.get(), 40);
    }

    #[test]
    fn resume_exclusion_allows_breaks_only_while_its_app_runs() {
        let mut sim =
            Sim::new(r#","appExclusions":[{"rule":"resume","active":true,"commands":["focus"]}]"#);
        sim.run_until(50_000);
        assert_eq!(sim.take_log(), log(&[]));

        sim.os.running = vec!["focus-timer.exe".into()];
        sim.run_until(75_000);
        assert_eq!(sim.take_log(), log(&[(72_000, "Mini break soon")]));
    }

    #[test]
    fn pause_resume_and_toggle_commands() {
        let mut sim = Sim::new("");
        sim.command(&["pause", "-d", "1h"]);
        assert_eq!(sim.core.tray(0), ("Paused, resumes in 1 hour".into(), true));
        sim.command(&["resume"]);
        assert!(!sim.core.tray(0).1);
        sim.command(&["toggle"]);
        assert_eq!(sim.core.tray(0), ("Paused".into(), true));
        sim.command(&["toggle"]);
        assert!(!sim.core.tray(0).1);

        sim.command(&["pause", "-d", "soon"]);
        sim.command(&["unknown"]);
        assert!(!sim.core.tray(0).1, "invalid input changes nothing");

        sim.os.morning = Some(60_000);
        sim.command(&["pause", "-d", "until-morning"]);
        assert_eq!(
            sim.core.tray(0),
            ("Paused, resumes in 1 minute".into(), true)
        );
    }

    #[test]
    fn long_command_waits_then_shows_the_given_words() {
        let mut sim = Sim::new("");
        sim.run_until(5_000);
        sim.command(&["long", "-w", "1", "-T", "Stretch up!", "-t", "Go stretch!"]);
        sim.run_until(70_000);
        assert_eq!(sim.take_log(), log(&[(65_000, "Long break: 15 seconds")]));
        assert_eq!(sim.last_text, "Stretch up!: Go stretch!");
    }

    #[test]
    fn mini_without_skip_sets_the_words_and_last_repeats_that_break() {
        let mut sim = Sim::new("");
        sim.command(&["mini", "-n", "-T", "Look away"]);
        sim.run_until(30_000);
        assert_eq!(sim.last_text, "Look away");

        sim.command(&["last"]);
        sim.run_until(45_000);
        assert_eq!(
            sim.take_log(),
            log(&[
                (20_000, "Mini break soon"),
                (30_000, "Mini break: 10 seconds"),
                (40_000, "Break over"),
            ]),
            "no repeat while a break runs"
        );
        sim.command(&["last"]);
        sim.run_until(45_100);
        assert_eq!(sim.take_log(), log(&[(45_100, "Mini break: 10 seconds")]));
        assert_eq!(sim.last_text, "Look away");
    }

    #[test]
    fn manual_finish_ends_the_break_at_once_until_the_window_exists() {
        let mut sim = Sim::new(r#","miniBreakManualFinish":true"#);
        sim.run_until(45_000);
        assert_eq!(
            sim.take_log(),
            log(&[
                (20_000, "Mini break soon"),
                (30_000, "Mini break: 10 seconds"),
                (40_000, "Break over")
            ])
        );
    }

    #[test]
    fn every_menu_id_leads_back_to_its_own_action() {
        for action in MenuAction::ALL {
            assert_eq!(MenuAction::from_id(action.id()), Some(action));
        }
    }
}
