//! The built-in break ideas: one action, what a study found about it, and the study.
//!
//! Each paper was published in 2016-2026 and checked against its abstract on 2026-11-10.
//! The findings retain the study limits. English only for now.

pub(crate) struct Evidence {
    pub(crate) action: &'static str,
    pub(crate) finding: &'static str,
    pub(crate) source: &'static str,
}

/// Ideas for Mini breaks, which last about 20 seconds.
pub(crate) const MINI: &[Evidence] = &[
    // DOI: 10.1016/j.clae.2022.101744
    Evidence {
        action: "Look at something 6 m away",
        finding: "In 29 symptomatic computer users, eye symptoms fell after two weeks of 20-second distance-break reminders. The improvement was not maintained after reminders stopped.",
        source: "Talens-Estarelles et al., 2023 · Contact Lens and Anterior Eye",
    },
    // DOI: 10.1016/j.clae.2020.04.014
    Evidence {
        action: "Blink gently and fully for 10 seconds",
        finding: "People with dry-eye symptoms reported improvement after four weeks of frequent 10-second blinking exercises. The study had no control group.",
        source: "Kim et al., 2021 · Contact Lens and Anterior Eye",
    },
    // DOI: 10.1016/j.exer.2025.110463
    Evidence {
        action: "Look away from your screen",
        finding: "During a 40-minute reading task, 24 young adults reported less eye strain with 20-second breaks every 10 minutes than with no breaks.",
        source: "Redondo et al., 2025 · Experimental Eye Research",
    },
    // DOI: 10.1007/s43674-024-00074-6
    Evidence {
        action: "Put your work aside for a moment",
        finding: "In a lab task, 20-second breaks lowered reported workload. The study found no significant difference in its concentration measure.",
        source: "Dianita et al., 2024 · Advances in Computational Intelligence",
    },
    // DOI: 10.5271/sjweh.3949
    Evidence {
        action: "Shift your sitting position",
        finding: "Over six months, at-risk office workers using a seat pad set to induce 20–60 shifts per hour reported fewer new cases of neck and back pain.",
        source: "Waongenngarm et al., 2021 · Scandinavian Journal of Work, Environment & Health, RCT",
    },
];

/// Ideas for Long breaks, which last about 5 minutes.
pub(crate) const LONG: &[Evidence] = &[
    // DOI: 10.1123/jpah.2016-0366
    Evidence {
        action: "Take a light two-minute walk",
        finding: "In 17 office workers, two-minute walks every 20 minutes lowered blood sugar after test drinks compared with uninterrupted sitting.",
        source: "Brocklebank et al., 2017 · Journal of Physical Activity and Health, RCT",
    },
    // DOI: 10.1152/japplphysiol.00796.2020
    Evidence {
        action: "Stand up from your chair 15 times",
        finding: "In 14 healthy adults, repeated chair stands with calf raises every 30 minutes lowered insulin after lunch compared with uninterrupted sitting.",
        source: "Gillen et al., 2021 · Journal of Applied Physiology, RCT",
    },
    // DOI: 10.1016/j.jesf.2024.03.004
    Evidence {
        action: "Walk up and down stairs for a minute",
        finding: "In 31 healthy young adults, one minute of comfortable stair walking after a meal lowered the short-term rise in blood sugar and insulin.",
        source: "Moore et al., 2024 · Journal of Exercise Science & Fitness, RCT",
    },
    // DOI: 10.1016/j.xcrm.2022.100895
    Evidence {
        action: "Breathe slowly, with longer exhales",
        finding: "In a remote trial, five minutes of daily exhale-focused breathing for a month improved mood more than mindfulness meditation.",
        source: "Yilmaz Balban et al., 2023 · Cell Reports Medicine, RCT",
    },
    // DOI: 10.2337/dc15-2336
    Evidence {
        action: "Do half-squats, calf raises, glute squeezes and knee raises",
        finding: "In 24 inactive adults with type 2 diabetes, three-minute sets of these movements every 30 minutes lowered blood sugar after meals compared with uninterrupted sitting.",
        source: "Dempsey et al., 2016 · Diabetes Care, randomized crossover trial",
    },
];
