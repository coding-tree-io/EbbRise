//! The built-in break ideas: one action, what a study found about it, and the study.
//!
//! Every entry was checked against the paper's abstract on 2026-11-09. The wording keeps
//! each study's limits: most effects are small or short-term. English only for now.

/// (action, finding, source) for Mini breaks, which last about 20 seconds.
pub(crate) const MINI: &[(&str, &str, &str)] = &[
    (
        "Look at something 6 m away",
        "In a two-week trial, reminders to look far for 20 seconds every 20 minutes reduced eye strain and dry-eye symptoms. The gain faded once the reminders stopped.",
        "Talens-Estarelles et al., 2023 · Contact Lens and Anterior Eye",
    ),
    (
        "Blink fully and slowly, 10 times",
        "People blinked about five times less often while using a screen. In a later study, more incomplete blinks went with more eye symptoms.",
        "Patel et al., 1991; Portello et al., 2013 · Optometry and Vision Science",
    ),
    (
        "Stand up and roll your shoulders",
        "Short breaks during keyboard work reduced discomfort in every body area measured, most with breaks every 20 minutes, with no loss of output.",
        "McLean et al., 2001 · Applied Ergonomics",
    ),
    (
        "Take your hands off the keyboard and look away",
        "Across 22 study samples, breaks of 10 minutes or less raised vigour and lowered fatigue. The effects were small.",
        "Albulescu et al., 2022 · PLOS ONE, meta-analysis",
    ),
];

/// (action, finding, source) for Long breaks, which last about 5 minutes.
pub(crate) const LONG: &[(&str, &str, &str)] = &[
    (
        "Walk for the rest of the break",
        "Two-minute walks every 20 minutes of sitting lowered blood sugar and insulin after a meal, compared with sitting without breaks.",
        "Dunstan et al., 2012 · Diabetes Care",
    ),
    (
        "Walk at an easy pace",
        "Three 5-minute walks during 3 hours of sitting prevented the drop in leg artery function that sitting alone caused.",
        "Thosar et al., 2015 · Medicine & Science in Sports & Exercise",
    ),
    (
        "Get up and move, often",
        "Short walks spread over the day improved mood and lowered fatigue and food cravings by the evening. One 30-minute walk in the morning did not.",
        "Bergouignan et al., 2016 · International Journal of Behavioral Nutrition and Physical Activity",
    ),
];
