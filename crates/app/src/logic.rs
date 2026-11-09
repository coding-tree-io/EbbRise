//! The app's behaviour without Windows: signals and menu choices go in with the time,
//! toasts and the next wake-up come out. `main.rs` only connects this to the OS.

mod break_screen;
mod command;
mod menu;
mod settings_form;
mod status;

use breaks::{BreakTime, BreaksPlanner, Event};
use fastrand::Rng;
use i18n::Translator;
use ideas::{BreakIdea, Ideas};
use platform::Signal;
use settings::Settings;

pub use break_screen::{BreakButton, BreakScreen, countdown, ring_commands};
pub use menu::{MenuAction, MenuEntry};
pub use settings_form::{BreakForm, SettingsForm};

/// How often to check for the user's return while they are away (no input event exists for that).
const AWAY_CHECK_MS: i64 = 2_000;
const FOCUS_ASSIST_UNSUPPORTED: i32 = -1;
/// The sound name that means "play nothing".
pub const SILENCE: &str = "silence";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub title: String,
    pub text: String,
}

/// What the app carries out after a signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Toast(Toast),
    /// Play the sound with this name, such as `crystal-glass`, at the settings' volume.
    Sound(String),
    /// Open the break window with these words.
    ShowBreak(BreakScreen),
    /// Close the break window.
    CloseBreak,
    /// Open the settings window (the `preferences` command).
    OpenSettings,
}

/// The next timer: fire after `delay` ms; Windows may shift it by `tolerance` ms to batch wake-ups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wake {
    pub delay: i64,
    pub tolerance: u32,
}

/// What the core asks the OS for only when it needs it.
pub trait Environment {
    /// Executable names of the running processes; `None` when Windows cannot list them.
    fn running_processes(&self) -> Option<Vec<String>>;
    /// Milliseconds until the configured morning; `None` when there is no morning time.
    fn until_morning(&self, settings: &Settings) -> Option<i64>;
}

/// The kind and words of the last break, for the `last` command.
#[derive(Clone, Debug)]
struct LastBreak {
    long: bool,
    idea: BreakIdea,
}

pub struct Core {
    planner: BreaksPlanner,
    words: Translator,
    ideas: Ideas,
    exclusion_checked_at: i64,
    /// Words that a `mini` or `long` command set for the next break (upstream `nextIdea`).
    next_idea: Option<command::IdeaOverride>,
    last_break: Option<LastBreak>,
}

impl Core {
    /// `focus_assist` is the value at start, `None` when Windows cannot report it.
    pub fn new(settings: Settings, now: i64, focus_assist: Option<i32>) -> Self {
        let words = Translator::new(&settings.language);
        let ideas = Ideas::load(&settings, Rng::new());
        let mut planner = BreaksPlanner::new(settings, now);
        planner.focus_assist(now, focus_assist.unwrap_or(FOCUS_ASSIST_UNSUPPORTED));
        Self {
            planner,
            words,
            ideas,
            exclusion_checked_at: now,
            next_idea: None,
            last_break: None,
        }
    }

    pub fn settings(&self) -> &Settings {
        self.planner.settings()
    }

    /// The texts in the user's language.
    pub fn words(&self) -> &Translator {
        &self.words
    }

    /// `now` is awake time and `idle` is time since the last input, both in ms.
    pub fn on_signal(
        &mut self,
        signal: Signal,
        now: i64,
        idle: i64,
        os: &impl Environment,
    ) -> Vec<Effect> {
        match signal {
            Signal::Timer => {
                self.planner.idle(now, idle);
                let mut effects = self.check_app_exclusion(now, os);
                effects.extend(self.run_due_steps(now));
                return effects;
            }
            Signal::Suspend | Signal::Lock => self.planner.suspend(now),
            Signal::Resume | Signal::Unlock => self.planner.wake(now),
            Signal::FocusAssist(value) => self.planner.focus_assist(now, value),
            Signal::Command(args) => return self.on_command(&args, now, os),
        }
        Vec::new()
    }

    /// When the next app exclusion check is due; `None` without an active rule or while
    /// paused, as upstream stops its poller. The first check after a pause is due at once.
    fn app_exclusion_check_due(&self) -> Option<i64> {
        let settings = self.settings();
        let active_rule = settings.app_exclusions.iter().any(|rule| rule.active);
        (active_rule && !self.planner.paused())
            .then(|| self.exclusion_checked_at + settings.app_exclusions_check_interval)
    }

    fn check_app_exclusion(&mut self, now: i64, os: &impl Environment) -> Vec<Effect> {
        if !self.app_exclusion_check_due().is_some_and(|due| due <= now) {
            return Vec::new();
        }
        self.exclusion_checked_at = now;
        // A failed list says nothing about the excluded app, so the hold stays as it is.
        let Some(names) = os.running_processes() else {
            return Vec::new();
        };
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let events = self.planner.processes_changed(now, &names);
        // A break that an exclusion ends gets no end sound, as upstream.
        events
            .into_iter()
            .flat_map(|event| self.carry_out(event, false))
            .collect()
    }

    /// Several steps can be due at once, for example after a long busy period.
    fn run_due_steps(&mut self, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        while self
            .planner
            .deadline()
            .is_some_and(|deadline| deadline <= now)
        {
            for event in self.planner.advance(now) {
                effects.extend(self.carry_out(event, true));
            }
        }
        effects
    }

    /// `on_time` is false when something other than the break's own time ended it.
    fn carry_out(&mut self, event: Event, on_time: bool) -> Vec<Effect> {
        let window = match event {
            Event::StartMiniBreak | Event::StartLongBreak => {
                let long = event == Event::StartLongBreak;
                let idea = self.take_idea(long);
                Some(Effect::ShowBreak(BreakScreen::new(&self.words, long, idea)))
            }
            Event::FinishMiniBreak | Event::FinishLongBreak => Some(Effect::CloseBreak),
            // The window stays and offers Finish (BreakTime::awaiting_finish).
            _ => None,
        };
        let sound = sound(self.settings(), &event, on_time).map(Effect::Sound);
        let toast = toast(&self.words, self.settings(), &event).map(Effect::Toast);
        toast.into_iter().chain(window).chain(sound).collect()
    }

    /// The running break for the break window; `None` outside a break.
    pub fn break_time(&self, now: i64) -> Option<BreakTime> {
        self.planner.break_time(now)
    }

    /// A break window button. Skip ends the break without its end sound, as upstream.
    pub fn on_break_button(&mut self, button: BreakButton, now: i64) -> Vec<Effect> {
        let Some(time) = self.planner.break_time(now) else {
            return Vec::new();
        };
        let offered = match button {
            BreakButton::Postpone => time.may_postpone,
            BreakButton::Skip => time.may_skip,
            BreakButton::Finish => time.awaiting_finish,
        };
        if !offered {
            return Vec::new();
        }
        if button == BreakButton::Postpone {
            self.planner.postpone(now);
            return vec![Effect::CloseBreak];
        }
        match self.planner.finish_break(now) {
            Some(event) => self.carry_out(event, button == BreakButton::Finish),
            None => Vec::new(),
        }
    }

    /// The words for a starting break: the `mini` or `long` command's words, else the next idea.
    fn take_idea(&mut self, long: bool) -> BreakIdea {
        let picked = if long {
            self.ideas.next_long()
        } else {
            self.ideas.next_mini()
        };
        let idea = match self.next_idea.take() {
            // The given words replace the whole idea: a built-in finding and source would not
            // match them.
            Some(words) => BreakIdea {
                title: words.title.unwrap_or_default(),
                text: words.text.unwrap_or_default(),
                source: String::new(),
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
            MenuAction::Pause(duration) => self.planner.pause(now, duration),
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
            MenuAction::Resume | MenuAction::Preferences | MenuAction::Quit => {}
        }
    }

    /// Applies settings changed in the settings window; a new language applies to new texts.
    pub fn set_settings(&mut self, settings: Settings, now: i64) {
        if settings.language != self.settings().language {
            self.words = Translator::new(&settings.language);
        }
        self.planner.set_settings(settings, now);
    }

    /// The tray menu for the current state.
    pub fn menu(&self, now: i64) -> Vec<MenuEntry> {
        menu::tray_menu(&self.planner, &self.words, now)
    }

    /// Upstream refuses to quit during a strict-mode break.
    pub fn may_quit(&self) -> bool {
        !menu::in_strict_break(&self.planner)
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
        let header = self.words.text("main.toolTipHeader");
        let message = status::tray_message(&self.planner, &self.words, now);
        let text = if message.is_empty() {
            header
        } else {
            format!(
                "{header}

{message}"
            )
        };
        (text, self.planner.paused())
    }
}

/// The start sound when a break starts, and the end sound when it ends on time.
fn sound(s: &Settings, event: &Event, on_time: bool) -> Option<String> {
    if s.silent_notifications {
        return None;
    }
    let name = match event {
        Event::StartMiniBreak => &s.mini_break_start_sound,
        Event::StartLongBreak => &s.long_break_start_sound,
        Event::FinishMiniBreak if on_time => &s.mini_break_audio,
        Event::FinishLongBreak if on_time => &s.long_break_audio,
        _ => return None,
    };
    (name != SILENCE).then(|| name.clone())
}

/// Upstream shows only the notifications and `resumingBreaks`; breaks get the break window.
fn toast(words: &Translator, s: &Settings, event: &Event) -> Option<Toast> {
    let seconds = |interval: i64| (interval / 1000).to_string();
    let (title, text) = match event {
        Event::MiniBreakNotification => (
            words.with(
                "main.miniBreakIn",
                &[("seconds", &seconds(s.mini_break_notification_interval))],
            ),
            String::new(),
        ),
        Event::LongBreakNotification => (
            words.with(
                "main.longBreakIn",
                &[("seconds", &seconds(s.long_break_notification_interval))],
            ),
            String::new(),
        ),
        Event::ResumeBreaks => (words.text("main.resumingBreaks"), String::new()),
        Event::StartMiniBreak
        | Event::StartLongBreak
        | Event::FinishMiniBreak
        | Event::FinishLongBreak
        | Event::AwaitMiniBreakFinish
        | Event::AwaitLongBreakFinish => return None,
    };
    Some(Toast { title, text })
}

#[cfg(test)]
mod tests {
    //! Discrete-event simulation: virtual time jumps straight to each wake-up the core asks for,
    //! so hours of app time run in microseconds. Expected times follow upstream breaksPlanner.js.

    use super::*;

    const USER_TIMER_MINIMUM_MS: i64 = 10;
    const HOUR_MS: i64 = 3_600_000;

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
        /// (time, title) for each toast and each break window.
        log: Vec<(i64, String)>,
        wakes: usize,
        os: FakeOs,
        /// Text of the last toast, or headline and detail of the last break window.
        last_text: String,
        window_open: bool,
        /// (time, sound name) for each sound.
        sounds: Vec<(i64, String)>,
    }

    #[derive(Default)]
    struct FakeOs {
        /// Process names Windows would list now; `None` when Windows refuses the list.
        running: Option<Vec<String>>,
        process_reads: std::cell::Cell<usize>,
        morning: Option<i64>,
    }

    impl Environment for FakeOs {
        fn running_processes(&self) -> Option<Vec<String>> {
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
                os: FakeOs {
                    running: Some(Vec::new()),
                    ..FakeOs::default()
                },
                last_text: String::new(),
                window_open: false,
                sounds: Vec::new(),
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
            let effects = self.core.on_signal(signal, self.now, self.idle(), &self.os);
            for effect in effects {
                self.record(effect);
            }
        }

        fn record(&mut self, effect: Effect) {
            match effect {
                Effect::Toast(toast) => {
                    self.log.push((self.now, toast.title));
                    self.last_text = toast.text;
                }
                Effect::Sound(name) => self.sounds.push((self.now, name)),
                Effect::ShowBreak(screen) => {
                    self.log.push((self.now, screen.kind));
                    self.last_text = [screen.headline, screen.detail]
                        .into_iter()
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()
                        .join(": ");
                    self.window_open = true;
                }
                Effect::CloseBreak => self.window_open = false,
                Effect::OpenSettings => self.log.push((self.now, "settings".to_owned())),
            }
        }

        fn press(&mut self, button: BreakButton) {
            for effect in self.core.on_break_button(button, self.now) {
                self.record(effect);
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

    /// The tooltip without its header line.
    fn status(core: &Core, now: i64) -> String {
        let (text, _) = core.tray(now);
        text.split_once(
            "

",
        )
        .map(|(_, message)| message.to_owned())
        .unwrap_or_default()
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
        assert_eq!(
            status(&sim.core, 0),
            "Next Mini break in about 1 minute
Next Long break in about 2 minutes"
        );
        sim.run_until(130_000);
        assert_eq!(
            sim.take_log(),
            log(&[
                (20_000, "Mini break in 10 seconds"),
                (30_000, "Mini break"),
                (60_000, "Mini break in 10 seconds"),
                (70_000, "Mini break"),
                (100_000, "Long break in 10 seconds"),
                (110_000, "Long break"),
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
        sim.core
            .on_menu(MenuAction::Pause(HOUR_MS), sim.now, &sim.os);
        assert_eq!(
            status(&sim.core, sim.now),
            "Paused - Resuming in about 1 hour"
        );
        sim.run_until(5_000 + HOUR_MS + 25_000);
        assert_eq!(
            sim.take_log(),
            log(&[
                (5_000 + HOUR_MS, "Resuming breaks"),
                (5_000 + HOUR_MS + 20_000, "Mini break in 10 seconds")
            ])
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
        assert_eq!(status(&sim.core, sim.now), "Paused Indefinitely");
        sim.core.on_menu(MenuAction::Resume, sim.now, &sim.os);
        assert!(!sim.core.tray(sim.now).1);
    }

    #[test]
    fn natural_break_stops_breaks_and_restarts_them_on_return() {
        let mut sim = Sim::new(r#","miniBreakInterval":600000"#);
        sim.away_since = Some(0);
        sim.run_until(400_000);
        assert_eq!(sim.take_log(), log(&[]));
        assert_eq!(status(&sim.core, sim.now), "", "nothing planned while away");

        sim.away_since = None;
        sim.run_until(400_000 + AWAY_CHECK_MS + 600_000);
        let log = sim.take_log();
        let (at, title) = &log[0];
        assert_eq!(title, "Mini break in 10 seconds");
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
        assert_eq!(
            sim.take_log(),
            log(&[(120_000, "Mini break in 10 seconds")])
        );
    }

    #[test]
    fn sleep_without_pause_keeps_the_remaining_time() {
        let mut sim = Sim::new(r#","pauseForSuspendOrLock":false"#);
        sim.run_until(5_000);
        // The awake clock stands still while the PC sleeps, so resume arrives at the same `now`.
        sim.signal(Signal::Suspend);
        sim.signal(Signal::Resume);
        sim.run_until(25_000);
        assert_eq!(sim.take_log(), log(&[(20_000, "Mini break in 10 seconds")]));
    }

    #[test]
    fn do_not_disturb_holds_breaks_until_it_ends() {
        let mut sim = Sim::new("");
        sim.signal(Signal::FocusAssist(2));
        sim.run_until(50_000);
        assert_eq!(sim.take_log(), log(&[]));
        sim.signal(Signal::FocusAssist(0));
        sim.run_until(75_000);
        assert_eq!(sim.take_log(), log(&[(70_000, "Mini break in 10 seconds")]));
    }

    #[test]
    fn pause_exclusion_holds_breaks_while_its_app_runs() {
        let mut sim =
            Sim::new(r#","appExclusions":[{"rule":"pause","active":true,"commands":["meeting"]}]"#);
        sim.os.running = Some(vec!["meeting.exe".into()]);
        sim.run_until(50_000);
        assert_eq!(sim.take_log(), log(&[]));

        sim.os.running = Some(Vec::new());
        sim.run_until(80_000);
        // The next 2 s check, at 52 s, sees the app gone and restarts the cycle.
        assert_eq!(sim.take_log(), log(&[(72_000, "Mini break in 10 seconds")]));
        assert_eq!(sim.os.process_reads.get(), 40);
    }

    #[test]
    fn exclusion_holds_when_the_process_list_fails_and_rests_during_a_pause() {
        let mut sim =
            Sim::new(r#","appExclusions":[{"rule":"pause","active":true,"commands":["meeting"]}]"#);
        sim.os.running = Some(vec!["meeting.exe".into()]);
        sim.run_until(10_000);
        sim.os.running = None;
        sim.run_until(50_000);
        assert_eq!(
            sim.take_log(),
            log(&[]),
            "a failed list is not a closed app"
        );

        sim.core
            .on_menu(MenuAction::PauseIndefinitely, sim.now, &sim.os);
        let reads = sim.os.process_reads.get();
        sim.run_until(100_000);
        assert_eq!(sim.os.process_reads.get(), reads, "no checks while paused");
    }

    #[test]
    fn resume_exclusion_allows_breaks_only_while_its_app_runs() {
        let mut sim =
            Sim::new(r#","appExclusions":[{"rule":"resume","active":true,"commands":["focus"]}]"#);
        sim.run_until(50_000);
        assert_eq!(sim.take_log(), log(&[]));

        sim.os.running = Some(vec!["focus-timer.exe".into()]);
        sim.run_until(75_000);
        assert_eq!(sim.take_log(), log(&[(72_000, "Mini break in 10 seconds")]));
    }

    #[test]
    fn pause_resume_and_toggle_commands() {
        let mut sim = Sim::new("");
        sim.command(&["pause", "-d", "1h"]);
        assert_eq!(status(&sim.core, 0), "Paused - Resuming in about 1 hour");
        sim.command(&["resume"]);
        assert!(!sim.core.tray(0).1);
        sim.command(&["toggle"]);
        assert_eq!(status(&sim.core, 0), "Paused Indefinitely");
        sim.command(&["toggle"]);
        assert!(!sim.core.tray(0).1);

        sim.command(&["pause", "-d", "soon"]);
        sim.command(&["unknown"]);
        assert!(!sim.core.tray(0).1, "invalid input changes nothing");

        sim.os.morning = Some(60_000);
        sim.command(&["pause", "-d", "until-morning"]);
        assert_eq!(status(&sim.core, 0), "Paused - Resuming in about 1 minute");
    }

    #[test]
    fn long_command_waits_then_shows_the_given_words() {
        let mut sim = Sim::new("");
        sim.run_until(5_000);
        sim.command(&["long", "-w", "1", "-T", "Stretch up!", "-t", "Go stretch!"]);
        sim.run_until(70_000);
        assert_eq!(sim.take_log(), log(&[(65_000, "Long break")]));
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
            log(&[(20_000, "Mini break in 10 seconds"), (30_000, "Mini break"),]),
            "no repeat while a break runs"
        );
        sim.command(&["last"]);
        sim.run_until(45_100);
        assert_eq!(sim.take_log(), log(&[(45_100, "Mini break")]));
        assert_eq!(sim.last_text, "Look away");
    }

    #[test]
    fn manual_finish_keeps_the_window_until_finish_is_pressed() {
        let sounds = r#","miniBreakManualFinish":true,"miniBreakAudio":"wind-chime""#;
        let mut sim = Sim::new(sounds);
        sim.run_until(50_000);
        assert!(sim.window_open && sim.core.break_time(sim.now).unwrap().awaiting_finish);
        sim.press(BreakButton::Finish);
        assert!(!sim.window_open);
        assert_eq!(sim.sounds.last(), Some(&(50_000, "wind-chime".to_owned())));
        sim.run_until(75_000);
        assert_eq!(
            sim.take_log(),
            log(&[
                (20_000, "Mini break in 10 seconds"),
                (30_000, "Mini break"),
                // Finished at 50 s, so the next break is 30 s later. The Long break, due at
                // 90 s, takes its place.
                (70_000, "Long break in 10 seconds"),
            ])
        );
    }

    #[test]
    fn commands_press_break_buttons_and_open_settings() {
        let mut sim = Sim::new(r#","miniBreakPostpone":false"#);
        sim.run_until(31_000);
        sim.command(&["skip"]);
        assert!(!sim.window_open);
        sim.command(&["preferences"]);
        assert_eq!(sim.log.last(), Some(&(31_000, "settings".to_owned())));
    }

    #[test]
    fn postpone_and_skip_close_the_window_only_when_offered() {
        let mut sim = Sim::new(r#","miniBreakPostpone":true,"miniBreakAudio":"wind-chime""#);
        sim.run_until(31_000);
        sim.press(BreakButton::Skip);
        assert!(sim.window_open, "no skip while postponing is offered");
        sim.press(BreakButton::Postpone);
        assert!(!sim.window_open && !sim.core.planner.in_break());

        let mut sim = Sim::new(r#","miniBreakPostpone":false,"miniBreakAudio":"wind-chime""#);
        sim.run_until(31_000);
        sim.press(BreakButton::Finish);
        assert!(sim.window_open, "finish waits for the end of the time");
        sim.press(BreakButton::Skip);
        assert!(!sim.window_open);
        assert!(sim.sounds.is_empty(), "skip plays no end sound");
    }

    #[test]
    fn breaks_play_their_start_sound_and_their_end_sound_when_on_time() {
        let sounds = r#","miniBreakStartSound":"tic-toc","miniBreakAudio":"wind-chime""#;
        let mut sim = Sim::new(sounds);
        sim.run_until(41_000);
        assert_eq!(
            sim.sounds,
            [
                (30_000, "tic-toc".to_owned()),
                (40_000, "wind-chime".to_owned())
            ]
        );

        // "silence" and silent notifications play nothing.
        let mut quiet = Sim::new(r#","miniBreakAudio":"silence""#);
        quiet.run_until(41_000);
        assert!(quiet.sounds.is_empty(), "{:?}", quiet.sounds);
        let mut silent = Sim::new(&format!("{sounds},\"silentNotifications\":true"));
        silent.run_until(41_000);
        assert!(silent.sounds.is_empty());
    }

    #[test]
    fn a_break_that_an_exclusion_ends_plays_no_end_sound() {
        let mut sim =
            Sim::new(r#","appExclusions":[{"rule":"pause","active":true,"commands":["meeting"]}]"#);
        sim.run_until(31_000);
        sim.os.running = Some(vec!["meeting.exe".into()]);
        sim.run_until(45_000);
        assert!(
            !sim.core.planner.in_break(),
            "the exclusion ended the break"
        );
        // The default start sound is "silence"; the default end sound would be "crystal-glass".
        assert!(sim.sounds.is_empty(), "{:?}", sim.sounds);
    }

    #[test]
    fn quit_waits_for_the_end_of_a_strict_break() {
        let mut sim = Sim::new(r#","miniBreakStrictMode":true"#);
        sim.run_until(31_000);
        assert!(!sim.core.may_quit());
        sim.run_until(41_000);
        assert!(sim.core.may_quit());
    }
}
