#[derive(Debug, Default, PartialEq, Eq)]
pub struct Command {
    pub name: String,
    pub title: Option<String>,
    pub text: Option<String>,
    pub no_skip: bool,
    pub wait: Option<String>,
    pub duration: Option<String>,
}

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
    pub fn pause_duration(&self, until_morning: i64) -> Option<i64> {
        match self.duration.as_deref() {
            None | Some("") | Some("indefinitely") => Some(1),
            Some("until-morning") => Some(until_morning),
            Some(value) => parse_duration(value),
        }
    }

    pub fn wait_duration(&self) -> Option<i64> {
        match self.wait.as_deref() {
            None | Some("") => Some(0),
            Some(value) => parse_duration(value),
        }
    }

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
