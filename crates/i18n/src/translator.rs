use icu_locale_core::Locale;
use icu_plurals::{PluralCategory, PluralRules};
use serde_json::Value;

use crate::locales::LOCALES;

const FALLBACK: &str = "en";
/// The app's own name replaces Stretchly's in every text.
const UPSTREAM_NAME: &str = "Stretchly";
const APP_NAME: &str = "EbbRise";
const NESTING_START: &str = "$t(";

/// One language of the app, with English for missing texts, as upstream's i18next setup.
///
/// Reads i18next JSON: dotted keys, `{{name}}` values, `_one`/`_other`/… plural keys and
/// `$t(key, {'count': n})` nesting.
pub struct Translator {
    /// `None` for English or a language without a file.
    language: Option<Source>,
    english: Source,
}

struct Source {
    texts: Value,
    plurals: Option<PluralRules>,
}

impl Source {
    fn load(code: &str) -> Option<Self> {
        let (_, json) = LOCALES.iter().find(|(name, _)| *name == code)?;
        let texts = serde_json::from_str(json).ok()?;
        let plurals = Locale::try_from_str(code)
            .ok()
            .and_then(|locale| PluralRules::try_new_cardinal(locale.into()).ok());
        Some(Self { texts, plurals })
    }

    fn find(&self, key: &str) -> Option<&str> {
        key.split('.')
            .try_fold(&self.texts, |value, part| value.get(part))?
            .as_str()
    }

    fn find_counted(&self, key: &str, count: Option<u64>) -> Option<&str> {
        let plural_key = count
            .zip(self.plurals.as_ref())
            .map(|(count, rules)| format!("{key}_{}", suffix(rules.category_for(count))));
        plural_key
            .and_then(|plural_key| self.find(&plural_key))
            .or_else(|| self.find(key))
    }
}

impl Translator {
    /// The translator for a language code such as `de` or `pt-BR`; English for an unknown code.
    ///
    /// # Panics
    /// When the built-in English file is broken, which the tests rule out.
    pub fn new(language: &str) -> Self {
        let english = Source::load(FALLBACK).expect("built-in English locale");
        let language = Source::load(language).filter(|_| language != FALLBACK);
        Self { language, english }
    }

    /// The text at a dotted `key`, such as `main.pause`; the key itself when no file has it.
    pub fn text(&self, key: &str) -> String {
        self.translate(key, None, &[])
    }

    /// The text at `key` in the chosen language only: `None` for English, or when that
    /// language's file lacks the key, so a caller can keep its own English wording.
    pub fn translated(&self, key: &str) -> Option<String> {
        let text = self.language.as_ref()?.find(key)?;
        Some(text.replace(UPSTREAM_NAME, APP_NAME))
    }

    /// The plural form of `key` for `count`, with `{{count}}` filled in.
    pub fn count(&self, key: &str, count: u64) -> String {
        self.translate(key, Some(count), &[])
    }

    /// The text at `key` with each `{{name}}` replaced by its value.
    pub fn with(&self, key: &str, values: &[(&str, &str)]) -> String {
        self.translate(key, None, values)
    }

    /// The child keys of the object at `path` in the English file, such as the idea ids.
    pub fn english_keys(&self, path: &str) -> Vec<String> {
        path.split('.')
            .try_fold(&self.english.texts, |value, part| value.get(part))
            .and_then(Value::as_object)
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// A duration to the nearest minute, such as "1 hour 5 minutes", as upstream's
    /// humanize-duration call with the units hours and minutes.
    pub fn duration(&self, milliseconds: i64) -> String {
        let minutes = (milliseconds.max(0) as f64 / 60_000.0).round() as u64;
        let (hours, minutes) = (minutes / 60, minutes % 60);
        match (hours, minutes) {
            (0, minutes) => self.count("utils.minutes", minutes),
            (hours, 0) => self.count("utils.hours", hours),
            (hours, minutes) => format!(
                "{} {}",
                self.count("utils.hours", hours),
                self.count("utils.minutes", minutes)
            ),
        }
    }

    fn translate(&self, key: &str, count: Option<u64>, values: &[(&str, &str)]) -> String {
        let Some(text) = self
            .language
            .iter()
            .chain([&self.english])
            .find_map(|source| source.find_counted(key, count))
        else {
            return key.to_owned();
        };
        let count = count.map(|count| count.to_string());
        let values: Vec<(&str, &str)> = count
            .as_deref()
            .map(|count| ("count", count))
            .into_iter()
            .chain(values.iter().copied())
            .collect();
        let text = self.nest(&interpolate(text, &values));
        text.replace(UPSTREAM_NAME, APP_NAME)
    }

    /// Replaces each `$t(key, {'count': n})` with that text. A nested text is not
    /// searched again, so a file cannot make this loop.
    fn nest(&self, text: &str) -> String {
        let mut result = String::new();
        let mut rest = text;
        while let Some(start) = rest.find(NESTING_START) {
            let call = &rest[start + NESTING_START.len()..];
            let Some(end) = call.find(')') else { break };
            let (key, options) = call[..end].split_once(',').unwrap_or((&call[..end], ""));
            let count = serde_json::from_str::<Value>(&options.replace('\'', "\""))
                .ok()
                .and_then(|options| options.get("count")?.as_u64());
            result.push_str(&rest[..start]);
            result.push_str(&self.translate(key.trim(), count, &[]));
            rest = &call[end + 1..];
        }
        result + rest
    }
}

/// Fills `{{name}}` and `{{name, format}}`; an unknown name stays as it is.
fn interpolate(text: &str, values: &[(&str, &str)]) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        let placeholder = &rest[start..start + end + 2];
        let name = placeholder[2..placeholder.len() - 2]
            .split(',')
            .next()
            .unwrap_or("");
        let value = values.iter().find(|(key, _)| *key == name.trim());
        result.push_str(&rest[..start]);
        result.push_str(value.map_or(placeholder, |(_, value)| value));
        rest = &rest[start + end + 2..];
    }
    result + rest
}

fn suffix(category: PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_values_plurals_and_nested_texts() {
        let english = Translator::new("en");
        assert_eq!(english.text("main.pause"), "Pause breaks");
        assert_eq!(english.count("utils.minutes", 1), "1 minute");
        assert_eq!(english.count("utils.minutes", 5), "5 minutes");
        assert_eq!(
            english.with("main.miniBreakIn", &[("seconds", "1")]),
            "Mini break in 1 second"
        );
        assert_eq!(english.text("main.quitStretchly"), "Quit EbbRise");
        assert_eq!(english.text("no.such.key"), "no.such.key");
    }

    #[test]
    fn uses_the_language_plural_rules_and_falls_back_to_english() {
        let russian = Translator::new("ru");
        assert_eq!(russian.count("utils.minutes", 1), "1 минута");
        assert_eq!(russian.count("utils.minutes", 3), "3 минуты");
        assert_eq!(russian.count("utils.minutes", 5), "5 минут");
        // The Basque file is empty, and an unknown code means English.
        assert_eq!(Translator::new("eu").text("main.pause"), "Pause breaks");
        assert_eq!(Translator::new("xx").text("main.pause"), "Pause breaks");
    }

    #[test]
    fn duration_rounds_to_minutes_and_names_hours() {
        let english = Translator::new("en");
        let cases = [
            (0, "0 minutes"),
            (29_000, "0 minutes"),
            (30_000, "1 minute"),
            (3_600_000, "1 hour"),
            (3_900_000, "1 hour 5 minutes"),
            (90_000_000, "25 hours"),
        ];
        for (milliseconds, expected) in cases {
            assert_eq!(
                english.duration(milliseconds),
                expected,
                "{milliseconds} ms"
            );
        }
    }

    const BACKEND_KEYS: [&str; 24] = [
        "main.toolTipHeader",
        "main.miniBreakIn",
        "main.longBreakIn",
        "main.resumingBreaks",
        "main.toMiniBreak",
        "main.toLongBreak",
        "main.skipToTheNext",
        "main.resume",
        "main.pause",
        "main.untilMorning",
        "main.indefinitely",
        "main.resetBreaks",
        "main.quitStretchly",
        "statusMessages.paused",
        "statusMessages.indefinitely",
        "statusMessages.dndMode",
        "statusMessages.appExclusion",
        "statusMessages.nextLongBreak",
        "statusMessages.nextMiniBreak",
        "statusMessages.afterMiniBreak_other",
        "statusMessages.resuming",
        "utils.inAbout",
        "utils.minutes_other",
        "utils.hours_other",
    ];

    /// A broken locale file would show raw keys in the tray, so every file must parse and
    /// every nested text must resolve.
    #[test]
    fn every_locale_parses_and_its_menu_texts_resolve() {
        let files = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/locales")).unwrap();
        assert_eq!(
            files.count(),
            LOCALES.len(),
            "a file in locales/ is not in LOCALES"
        );
        for (code, json) in LOCALES {
            serde_json::from_str::<Value>(json).unwrap_or_else(|error| panic!("{code}: {error}"));
            let translator = Translator::new(code);
            let text = translator.with("main.longBreakIn", &[("seconds", "2")]);
            assert!(
                !text.contains("$t(") && !text.contains("{{"),
                "{code}: {text}"
            );
            for key in BACKEND_KEYS {
                assert_ne!(translator.text(key), key, "{code}: {key}");
            }
        }
    }
}
