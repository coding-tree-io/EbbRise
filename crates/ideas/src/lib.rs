//! Break ideas: the words a break shows, picked as upstream's `loadIdeas` and `Shuffled`.
//! The built-in ideas are our own evidence-based set, not Stretchly's.

mod evidence;

use fastrand::Rng;
use settings::Settings;

/// The words a break shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BreakIdea {
    /// The action, shown as the headline; may be empty for the user's own Mini break ideas.
    pub title: String,
    /// What the break is about: the finding behind the action, or the user's own words.
    pub text: String,
    /// The study behind the idea; empty for the user's own ideas.
    pub source: String,
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
    /// The user's enabled ideas when `useIdeasFromSettings` is on, else the built-in
    /// evidence-based ideas. With `ideas` off, every break gets empty words.
    pub fn load(settings: &Settings, mut rng: Rng) -> Self {
        let (mini, long) = match (settings.ideas, settings.use_ideas_from_settings) {
            (false, _) => (Vec::new(), Vec::new()),
            (true, true) => from_settings(settings),
            (true, false) => built_in(),
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
            text: idea.data.clone(),
            ..BreakIdea::default()
        });
    let long = settings
        .long_break_ideas
        .iter()
        .filter(|idea| idea.enabled)
        .map(|idea| BreakIdea {
            title: idea.data.first().cloned().unwrap_or_default(),
            text: idea.data.get(1).cloned().unwrap_or_default(),
            source: String::new(),
        });
    (mini.collect(), long.collect())
}

/// The evidence-based ideas in [`evidence`]: the action as the title, the finding as the text.
fn built_in() -> (Vec<BreakIdea>, Vec<BreakIdea>) {
    let ideas = |entries: &[(&str, &str, &str)]| {
        entries
            .iter()
            .map(|(title, text, source)| BreakIdea {
                title: (*title).to_owned(),
                text: (*text).to_owned(),
                source: (*source).to_owned(),
            })
            .collect()
    };
    (ideas(evidence::MINI), ideas(evidence::LONG))
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
        let mut ideas = Ideas::load(&settings, Rng::with_seed(7));
        for _ in 0..3 {
            let mut round = mini_texts(&mut ideas, 3);
            round.sort();
            assert_eq!(round, ["a", "b", "c"]);
        }
    }

    #[test]
    fn every_built_in_idea_names_its_action_finding_and_source() {
        let mut ideas = Ideas::load(&Settings::default(), Rng::with_seed(1));
        let mut all: Vec<BreakIdea> = (0..evidence::MINI.len())
            .map(|_| ideas.next_mini())
            .collect();
        all.extend((0..evidence::LONG.len()).map(|_| ideas.next_long()));
        for idea in all {
            assert!(!idea.title.is_empty() && !idea.text.is_empty() && !idea.source.is_empty());
        }
    }

    #[test]
    fn ideas_off_gives_empty_words() {
        let settings = Settings {
            ideas: false,
            ..Settings::default()
        };
        let mut ideas = Ideas::load(&settings, Rng::with_seed(1));
        assert_eq!(ideas.next_long(), BreakIdea::default());
        assert_eq!(ideas.next_mini(), BreakIdea::default());
    }
}
