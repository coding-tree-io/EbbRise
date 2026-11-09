//! Commands that a second `stretch` process forwards, as upstream's `second-instance` handler.

use breaks::BreaksPlanner;
use commands::Command;

use super::{BreakIdea, Core, Environment};

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
    /// Runs a forwarded command line. `help`, `version` and `logs` run in the second
    /// process. `postpone`, `skip` and `finish` need a break window, and upstream also drops
    /// them when none is open. `preferences` waits for the settings window, which upstream
    /// opens.
    pub(super) fn on_command(&mut self, args: &[String], now: i64, os: &impl Environment) {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(command) = Command::parse(&args) else {
            return;
        };
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
    }

    fn pause_from_command(&mut self, command: &Command, now: i64, os: &impl Environment) {
        let until_morning = os
            .until_morning(self.settings())
            .unwrap_or(BreaksPlanner::INDEFINITELY);
        if let Some(duration) = command.pause_duration(until_morning) {
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
