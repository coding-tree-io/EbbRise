//! Commands that a second `stretch` process forwards, as upstream's `second-instance` handler.

use breaks::BreaksPlanner;
use commands::Command;

use super::{BreakButton, BreakIdea, Core, Effect, Environment, MenuAction};

/// Words a `mini` or `long` command gives the next break; `None` keeps the picked idea's part.
#[derive(Clone, Debug, Default)]
pub(super) struct IdeaOverride {
    pub(super) title: Option<String>,
    pub(super) text: Option<String>,
}

impl From<BreakIdea> for IdeaOverride {
    fn from(idea: BreakIdea) -> Self {
        Self {
            title: Some(idea.title),
            text: Some(idea.text),
        }
    }
}

impl Core {
    pub(super) fn recheck_morning_pause(&mut self, now: i64, os: &impl Environment) {
        let Some(target) = self.morning_target else {
            return;
        };
        if !self.planner.paused() {
            self.morning_target = None;
        } else if target <= os.wall_ms() {
            self.planner.resume(now);
            self.morning_target = None;
        } else {
            let left = (target - os.wall_ms()).max(BreaksPlanner::INDEFINITELY + 1);
            self.planner.pause(now, left);
        }
    }

    /// Pause until morning pauses indefinitely when there is no morning time.
    pub fn on_menu(&mut self, action: MenuAction, now: i64, os: &impl Environment) -> Vec<Effect> {
        if matches!(
            action,
            MenuAction::Resume
                | MenuAction::Pause(_)
                | MenuAction::PauseUntilMorning
                | MenuAction::PauseIndefinitely
                | MenuAction::Reset
        ) {
            self.morning_target = None;
        }
        match action {
            MenuAction::Resume if self.planner.paused() => self.planner.resume(now),
            MenuAction::Pause(duration) => self.planner.pause(now, duration),
            MenuAction::PauseUntilMorning => {
                self.pause_until_morning(now, os);
            }
            MenuAction::PauseIndefinitely => self.planner.pause(now, BreaksPlanner::INDEFINITELY),
            MenuAction::SkipToMini => self.planner.skip_to_mini(now, None),
            MenuAction::SkipToLong => self.planner.skip_to_long(now, None),
            MenuAction::Reset => self.planner.reset(now),
            MenuAction::Resume
            | MenuAction::Preferences
            | MenuAction::Statistics
            | MenuAction::About
            | MenuAction::DownloadUpdate
            | MenuAction::CheckForUpdates
            | MenuAction::Quit => {}
        }
        self.interrupted_media().into_iter().collect()
    }

    fn pause_until_morning(&mut self, now: i64, os: &impl Environment) {
        let duration = os.until_morning(self.settings());
        self.morning_target = duration.map(|duration| os.wall_ms() + duration);
        self.planner
            .pause(now, duration.unwrap_or(BreaksPlanner::INDEFINITELY));
    }

    /// Runs a forwarded command line. `help`, `version` and `logs` run in the second
    /// process. `postpone`, `skip` and `finish` press the break window's button, and do
    /// nothing when it does not offer it.
    pub(super) fn on_command(
        &mut self,
        args: &[String],
        now: i64,
        os: &impl Environment,
    ) -> Vec<Effect> {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(command) = Command::parse(&args) else {
            return Vec::new();
        };
        let button = match command.name.as_str() {
            "postpone" => Some(BreakButton::Postpone),
            "skip" => Some(BreakButton::Skip),
            "finish" => Some(BreakButton::Finish),
            _ => None,
        };
        if let Some(button) = button {
            return self.on_break_button(button, now);
        }
        if command.name == "preferences" {
            return vec![Effect::OpenSettings];
        }
        if command.name == "statistics" {
            return vec![Effect::OpenStatistics];
        }
        if matches!(command.name.as_str(), "resume" | "toggle" | "reset") {
            self.morning_target = None;
        }
        match command.name.as_str() {
            "reset" => self.planner.reset(now),
            "pause" => self.pause_from_command(&command, now, os),
            "resume" if self.planner.paused() => self.planner.resume(now),
            "toggle" if self.planner.paused() => self.planner.resume(now),
            "toggle" => self.planner.pause(now, BreaksPlanner::INDEFINITELY),
            "mini" | "long" => self.skip_from_command(&command, now),
            "last" => self.start_last_break_again(now),
            _ => {}
        }
        Vec::new()
    }

    fn pause_from_command(&mut self, command: &Command, now: i64, os: &impl Environment) {
        if command.duration.as_deref() == Some("until-morning") {
            self.pause_until_morning(now, os);
            return;
        }
        let until_morning = os
            .until_morning(self.settings())
            .unwrap_or(BreaksPlanner::INDEFINITELY);
        if let Some(duration) = command.pause_duration(until_morning) {
            self.morning_target = None;
            self.planner.pause(now, duration);
        }
    }

    /// `-w` delays the break; `-n` only sets the words, unless a delay is given.
    fn skip_from_command(&mut self, command: &Command, now: i64) {
        let Some(wait) = command.wait_duration() else {
            return;
        };
        let long = command.name == "long";
        if long {
            self.next_idea = Some(IdeaOverride {
                title: command.title.clone(),
                text: command.text.clone(),
            });
        } else if let Some(title) = &command.title {
            // A Mini break has no title line, so upstream shows the given title as its text.
            self.next_idea = Some(IdeaOverride {
                title: None,
                text: Some(title.clone()),
            });
        }
        if command.no_skip && wait == 0 {
            return;
        }
        let delay = (wait > 0).then_some(wait);
        if long {
            self.planner.skip_to_long(now, delay);
        } else {
            self.planner.skip_to_mini(now, delay);
        }
    }

    /// Starts the last break again with the same words, unless a break runs or breaks are held.
    fn start_last_break_again(&mut self, now: i64) {
        let held = self.planner.paused() || self.planner.deadline().is_none();
        let Some(last) = self.last_break.clone() else {
            return;
        };
        if held || self.planner.in_break() {
            return;
        }
        self.next_idea = Some(last.idea.into());
        if last.long {
            self.planner.skip_to_long(now, None);
        } else {
            self.planner.skip_to_mini(now, None);
        }
    }
}
