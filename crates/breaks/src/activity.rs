//! Whether the user is using the PC, from the time since the last input
//! (`docs/workrave-features.md`, "Activity"). Sans-IO: the caller reads the idle time.

/// Input within this time counts as use; the user is idle after it.
pub const IDLE_AFTER_MS: i64 = 5_000;
/// After an idle period, a return counts only when more input comes at least this long after
/// the first input; a single bump of the mouse is noise.
pub const NOISE_MS: i64 = 1_000;
/// How often to look for the user's return while they are idle.
const IDLE_CHECK_MS: i64 = 2_000;
/// The shortest wait between two looks, so a wrong clock cannot make a busy loop.
const MIN_CHECK_MS: i64 = 200;

/// A change the monitor saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// The user is idle from `since`, 5 s after the last input.
    BecameIdle {
        /// Start of the idle time.
        since: i64,
    },
    /// The user came back with input at `at`.
    Returned {
        /// Time of the first input of the return.
        at: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Active {
        since: i64,
        last_input: i64,
    },
    Idle {
        last_input: i64,
        first_new_input: Option<i64>,
    },
}

/// Follows the user's activity and adds up their active time.
///
/// ```
/// use breaks::{Activity, ActivityMonitor};
/// let mut monitor = ActivityMonitor::new(0, 0);
/// assert_eq!(monitor.observe(10_000, 7_000), Some(Activity::BecameIdle { since: 8_000 }));
/// assert_eq!(monitor.active_time(20_000), 8_000);
/// ```
#[derive(Clone, Debug)]
pub struct ActivityMonitor {
    state: State,
    /// Active time of the finished active periods.
    finished_ms: i64,
}

impl ActivityMonitor {
    /// Starts at `now`; `idle` is the time since the last input. Active time counts from `now`.
    pub fn new(now: i64, idle: i64) -> Self {
        let last_input = now - idle;
        let state = if idle < IDLE_AFTER_MS {
            State::Active {
                since: now,
                last_input,
            }
        } else {
            State::Idle {
                last_input,
                first_new_input: None,
            }
        };
        Self {
            state,
            finished_ms: 0,
        }
    }

    /// The user is idle now.
    pub fn idle(&self) -> bool {
        matches!(self.state, State::Idle { .. })
    }

    /// Active time since the monitor started, up to `now`.
    pub fn active_time(&self, now: i64) -> i64 {
        match self.state {
            State::Active { since, .. } => self.finished_ms + (now - since).max(0),
            State::Idle { .. } => self.finished_ms,
        }
    }

    /// Takes a reading: `idle` is the time since the last input at `now`.
    ///
    /// `now` excludes sleep but `idle` does not, so after sleep the last input seems to come
    /// before an input already seen; the last input seen then stands.
    pub fn observe(&mut self, now: i64, idle: i64) -> Option<Activity> {
        let last = now - idle;
        match self.state {
            State::Active { since, last_input } => {
                let last = last.max(last_input);
                if now - last < IDLE_AFTER_MS {
                    self.state = State::Active {
                        since,
                        last_input: last,
                    };
                    return None;
                }
                let end = (last + IDLE_AFTER_MS).max(since);
                self.finished_ms += end - since;
                self.state = State::Idle {
                    last_input: last,
                    first_new_input: None,
                };
                Some(Activity::BecameIdle { since: end })
            }
            State::Idle {
                last_input,
                first_new_input: None,
            } => {
                if last > last_input {
                    self.state = State::Idle {
                        last_input,
                        first_new_input: Some(last),
                    };
                }
                None
            }
            State::Idle {
                first_new_input: Some(first),
                ..
            } => {
                if last >= first + NOISE_MS {
                    self.state = State::Active {
                        since: first,
                        last_input: last,
                    };
                    return Some(Activity::Returned { at: first });
                }
                if idle >= IDLE_AFTER_MS {
                    // One input and nothing after it: noise. It counts as seen, so it cannot
                    // start a return later.
                    self.state = State::Idle {
                        last_input: last,
                        first_new_input: None,
                    };
                }
                None
            }
        }
    }

    /// Time until the next reading can see a change.
    pub fn next_check(&self, now: i64, idle: i64) -> i64 {
        let delay = match self.state {
            State::Active { .. } => IDLE_AFTER_MS - idle,
            State::Idle {
                first_new_input: Some(first),
                ..
            } => first + NOISE_MS + MIN_CHECK_MS - now,
            State::Idle { .. } => IDLE_CHECK_MS,
        };
        delay.max(MIN_CHECK_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_starts_five_seconds_after_the_last_input_even_across_sleep() {
        let mut monitor = ActivityMonitor::new(0, 0);
        assert_eq!(monitor.observe(4_000, 4_000), None);
        assert_eq!(monitor.next_check(4_000, 4_000), 1_000);
        assert_eq!(
            monitor.observe(6_000, 6_000),
            Some(Activity::BecameIdle { since: 5_000 })
        );
        assert!(monitor.idle());
        assert_eq!(monitor.active_time(60_000), 5_000);

        // Input at 3 s, then an hour of sleep: the idle time includes the sleep, awake time not.
        let mut monitor = ActivityMonitor::new(0, 0);
        assert_eq!(monitor.observe(3_000, 0), None);
        assert_eq!(
            monitor.observe(10_000, 3_600_000),
            Some(Activity::BecameIdle { since: 8_000 })
        );
        assert_eq!(monitor.active_time(10_000), 8_000);
    }

    #[test]
    fn a_return_needs_input_one_second_after_the_first() {
        let mut monitor = ActivityMonitor::new(0, 10_000);
        assert!(monitor.idle());
        // A single bump at 20 s, nothing after it.
        assert_eq!(monitor.observe(21_000, 1_000), None);
        assert_eq!(monitor.observe(22_400, 2_400), None);
        assert_eq!(monitor.observe(26_000, 6_000), None, "noise is dropped");
        // A second bump at 40 s is noise too; with the first it is no return.
        assert_eq!(monitor.observe(28_000, 8_000), None);
        assert_eq!(monitor.observe(40_500, 500), None);
        assert_eq!(monitor.observe(46_000, 6_000), None);
        // Real use from 50 s.
        assert_eq!(monitor.observe(50_500, 0), None);
        assert_eq!(monitor.next_check(50_500, 0), 1_200);
        assert_eq!(
            monitor.observe(51_700, 0),
            Some(Activity::Returned { at: 50_500 })
        );
        assert_eq!(monitor.active_time(60_000), 9_500);
    }
}
