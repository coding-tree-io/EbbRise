//! Break ideas: the words a break shows, picked as upstream's `loadIdeas` and `Shuffled`.

use fastrand::Rng;
use i18n::Translator;
use settings::Settings;

/// The words a break shows. A Mini break shows only `text`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BreakIdea {
    /// Long break title; empty for a Mini break.
    pub title: String,
    /// The idea itself.
    pub text: String,
}

/// Items in random order; no item comes again before every other item came once.
struct Deck {
    all: Vec<BreakIdea>,
    left: Vec<BreakIdea>,
    rng: Rng,
}

impl Deck {
    fn new(all: Vec<BreakIdea>, rng: Rng) -> Self {
        Self {
            left: all.clone(),
            all,
            rng,
        }
    }

    fn draw(&mut self) -> BreakIdea {
        if self.left.is_empty() {
            self.left.clone_from(&self.all);
        }
        if self.left.is_empty() {
            return BreakIdea::default();
        }
        let index = self.rng.usize(..self.left.len());
        self.left.swap_remove(index)
    }
}

/// The Mini and Long break ideas the app picks from.
pub struct Ideas {
    mini: Deck,
    long: Deck,
}

impl Ideas {
    /// The user's enabled ideas when `useIdeasFromSettings` is on, else the built-in ideas in
    /// the translator's language. With `ideas` off, every break gets empty words.
    pub fn load(settings: &Settings, translator: &Translator, mut rng: Rng) -> Self {
        let (mini, long) = match (settings.ideas, settings.use_ideas_from_settings) {
            (false, _) => (Vec::new(), Vec::new()),
            (true, true) => from_settings(settings),
            (true, false) => built_in(translator),
        };
        Self {
            mini: Deck::new(mini, rng.fork()),
            long: Deck::new(long, rng),
        }
    }

    /// The next Mini break idea; empty when there is none.
    pub fn next_mini(&mut self) -> BreakIdea {
        self.mini.draw()
    }

    /// The next Long break idea; empty when there is none.
    pub fn next_long(&mut self) -> BreakIdea {
        self.long.draw()
    }
}

fn from_settings(settings: &Settings) -> (Vec<BreakIdea>, Vec<BreakIdea>) {
    let mini = settings
        .mini_break_ideas
        .iter()
        .filter(|idea| idea.enabled)
        .map(|idea| BreakIdea {
            title: String::new(),
            text: idea.data.clone(),
        });
    let long = settings
        .long_break_ideas
        .iter()
        .filter(|idea| idea.enabled)
        .map(|idea| BreakIdea {
            title: idea.data.first().cloned().unwrap_or_default(),
            text: idea.data.get(1).cloned().unwrap_or_default(),
        });
    (mini.collect(), long.collect())
}

/// The ids come from the English file, so a language with fewer ideas shows the missing
/// ones in English, as upstream.
fn built_in(translator: &Translator) -> (Vec<BreakIdea>, Vec<BreakIdea>) {
    let mini = translator
        .english_keys("miniBreakIdeas")
        .into_iter()
        .map(|id| BreakIdea {
            title: String::new(),
            text: translator.text(&format!("miniBreakIdeas.{id}.text")),
        });
    let long = translator
        .english_keys("longBreakIdeas")
        .into_iter()
        .map(|id| BreakIdea {
            title: translator.text(&format!("longBreakIdeas.{id}.title")),
            text: translator.text(&format!("longBreakIdeas.{id}.text")),
        });
    (mini.collect(), long.collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings::Idea;

    fn mini_texts(ideas: &mut Ideas, count: usize) -> Vec<String> {
        (0..count).map(|_| ideas.next_mini().text).collect()
    }

    #[test]
    fn every_enabled_idea_comes_once_per_round() {
        let settings = Settings {
            use_ideas_from_settings: true,
            mini_break_ideas: ["a", "b", "off", "c"]
                .map(|text| Idea {
                    data: text.to_owned(),
                    enabled: text != "off",
                })
                .into(),
            ..Settings::default()
        };
        let mut ideas = Ideas::load(&settings, &Translator::new("en"), Rng::with_seed(7));
        for _ in 0..3 {
            let mut round = mini_texts(&mut ideas, 3);
            round.sort();
            assert_eq!(round, ["a", "b", "c"]);
        }
    }

    #[test]
    fn built_in_ideas_follow_the_language_and_fill_gaps_in_english() {
        let english = Translator::new("en");
        let english_count = english.english_keys("longBreakIdeas").len();
        // Polish translates fewer Long break ideas than English has.
        let mut ideas = Ideas::load(
            &Settings::default(),
            &Translator::new("pl"),
            Rng::with_seed(1),
        );
        let round: Vec<BreakIdea> = (0..english_count).map(|_| ideas.next_long()).collect();
        let in_english = |idea: &&BreakIdea| {
            english
                .english_keys("longBreakIdeas")
                .iter()
                .any(|id| english.text(&format!("longBreakIdeas.{id}.title")) == idea.title)
        };
        let english_titles = round.iter().filter(in_english).count();
        assert!(english_titles > 0 && english_titles < english_count);
        assert!(
            round
                .iter()
                .all(|idea| !idea.title.is_empty() && !idea.text.is_empty())
        );
    }

    #[test]
    fn ideas_off_gives_empty_words() {
        let settings = Settings {
            ideas: false,
            ..Settings::default()
        };
        let mut ideas = Ideas::load(&settings, &Translator::new("en"), Rng::with_seed(1));
        assert_eq!(ideas.next_long(), BreakIdea::default());
        assert_eq!(ideas.next_mini(), BreakIdea::default());
    }
}
