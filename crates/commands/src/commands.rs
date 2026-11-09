/// Usage text for `ebbrise help`, with upstream's commands, options and examples.
pub const HELP: &str = "\
Usage: ebbrise <command> [options]

Commands:
	ebbrise help                  Show this help message
	ebbrise version               Show current EbbRise version
	ebbrise logs                  Show location of logs file
	ebbrise reset                 Reset breaks
	ebbrise pause [options]       Pause breaks
	ebbrise resume                Resume from a pause
	ebbrise toggle                Pause/unpause breaks
	ebbrise postpone              Postpone the currently running break, when allowed
	ebbrise skip                  Skip the currently running break, when allowed
	ebbrise finish                Finish the currently running break, when allowed
	ebbrise mini [options]        Skip to the Mini break, customize it
	ebbrise long [options]        Skip to the Long break, customize it
	ebbrise last                  Start the last Mini or Long break again
	ebbrise preferences           Open Preferences window

Options:
	-T, --title    Specify title for next break (Long or Mini)
	-t, --text     Specify text for next break (Long break only)
	-n, --noskip   Do not skip directly to this break (Long or Mini)
	-w, --wait     Specify an interval to wait before skipping to this break (Long or Mini) [HHhMMm|HHh|MMm|MM]
	-d, --duration Specify duration for pausing breaks (Pause only) [indefinitely|until-morning|HHhMMm|HHh|MMm|MM]

Examples:
	ebbrise pause                                    Pause breaks indefinitely
	ebbrise pause -d 60                              Pause breaks for one hour
	ebbrise pause -d 1h                              Pause breaks for one hour
	ebbrise pause -d 1h20m                           Pause breaks for one hour and twenty minutes
	ebbrise postpone                                 Postpone the currently running break using existing preferences
	ebbrise skip                                     Skip the currently running break using existing preferences
	ebbrise finish                                   Finish the break after its countdown in Manual Finish mode
	ebbrise mini -T \"Stretch up!\"                    Start a Mini break, with the title \"Stretch up!\"
	ebbrise long -T \"Stretch up!\" --noskip           Set the next break's title to \"Stretch up!\"
	ebbrise long -T \"Stretch up!\" -t \"Go stretch!\"   Start a long break, with the title \"Stretch up!\" and text \"Go stretch!\"
	ebbrise long -w 20m -T \"Stretch up!\"             Wait 20 minutes, then start a long break with the title set to \"Stretch up!\"
	ebbrise last                                     Start the last break again, with the same idea
	ebbrise preferences                              Open Preferences window";

/// A parsed command line; options that the command does not take are ignored, as upstream.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Command {
    /// Command name, such as `pause` or `mini`; `help` when none is given.
    pub name: String,
    /// `-T`/`--title` of `mini` and `long`.
    pub title: Option<String>,
    /// `-t`/`--text` of `long`.
    pub text: Option<String>,
    /// `-n`/`--noskip` of `mini` and `long`: set the next break instead of starting it.
    pub no_skip: bool,
    /// `-w`/`--wait` of `mini` and `long`, unparsed; see [`Command::wait_duration`].
    pub wait: Option<String>,
    /// `-d`/`--duration` of `pause`, unparsed; see [`Command::pause_duration`].
    pub duration: Option<String>,
}

/// Parses `HHhMMm`, `HHh`, `MMm` or `MM` (minutes) into milliseconds.
///
/// `"1h20m"` gives 4 800 000. Returns `None` for any other text and for zero.
pub fn parse_duration(input: &str) -> Option<i64> {
    let input = input.to_ascii_lowercase();
    let number = |s: &str| -> Option<i64> {
        if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
            None
        } else {
            s.parse().ok()
        }
    };
    let minutes = if let Some((hours, rest)) = input.split_once('h') {
        let hours = number(hours)?;
        let minutes = if rest.is_empty() {
            0
        } else {
            number(rest.strip_suffix('m')?)?
        };
        hours.checked_mul(60)?.checked_add(minutes)?
    } else if let Some(minutes) = input.strip_suffix('m') {
        number(minutes)?
    } else {
        number(&input)?
    };
    minutes.checked_mul(60_000).filter(|value| *value > 0)
}

impl Command {
    /// Pause length in ms for `pause`: 1 means indefinitely (Stretchly's value).
    ///
    /// `until_morning` is used for `-d until-morning`. Returns `None` for an invalid duration.
    pub fn pause_duration(&self, until_morning: i64) -> Option<i64> {
        match self.duration.as_deref() {
            None | Some("") | Some("indefinitely") => Some(1),
            Some("until-morning") => Some(until_morning),
            Some(value) => parse_duration(value),
        }
    }

    /// Delay in ms before `mini` or `long` starts; 0 when not given, `None` when invalid.
    pub fn wait_duration(&self) -> Option<i64> {
        match self.wait.as_deref() {
            None | Some("") => Some(0),
            Some(value) => parse_duration(value),
        }
    }

    /// Parses arguments without the program name; leading `--flags` (such as Electron's) are skipped.
    ///
    /// Returns `None` for an unknown command.
    pub fn parse(input: &[&str]) -> Option<Self> {
        let args = input
            .iter()
            .copied()
            .skip_while(|arg| arg.starts_with("--"))
            .collect::<Vec<_>>();
        let name = args.first().copied().unwrap_or("help");
        if ![
            "help",
            "version",
            "logs",
            "reset",
            "pause",
            "resume",
            "toggle",
            "postpone",
            "skip",
            "finish",
            "mini",
            "long",
            "last",
            "preferences",
        ]
        .contains(&name)
        {
            return None;
        }
        let mut result = Self {
            name: name.into(),
            ..Self::default()
        };
        let mut args = args.iter().skip(1);
        while let Some(arg) = args.next() {
            match *arg {
                "-T" | "--title" if name == "mini" || name == "long" => {
                    result.title = args.next().map(|v| (*v).into())
                }
                "-t" | "--text" if name == "long" => result.text = args.next().map(|v| (*v).into()),
                "-n" | "--noskip" if name == "mini" || name == "long" => result.no_skip = true,
                "-w" | "--wait" if name == "mini" || name == "long" => {
                    result.wait = args.next().map(|v| (*v).into())
                }
                "-d" | "--duration" if name == "pause" => {
                    result.duration = args.next().map(|v| (*v).into())
                }
                _ => {}
            }
        }
        Some(result)
    }
}
