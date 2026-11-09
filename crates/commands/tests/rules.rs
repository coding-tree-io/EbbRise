//! Command-line grammar and duration format.

#[test]
fn command_grammar_and_duration_table() {
    for (input, expected) in [
        ("60", Some(3_600_000)),
        ("4h39m", Some(16_740_000)),
        ("1H", Some(3_600_000)),
        ("20m", Some(1_200_000)),
        ("0", None),
        ("foo4h", None),
    ] {
        assert_eq!(commands::parse_duration(input), expected, "{input}");
    }
    let command =
        commands::Command::parse(&["--hidden", "mini", "-T", "Stretch", "-n", "-w", "20m"])
            .unwrap();
    assert_eq!(command.name, "mini");
    assert_eq!(command.title.as_deref(), Some("Stretch"));
    assert!(command.no_skip);
    assert_eq!(command.wait.as_deref(), Some("20m"));
    assert_eq!(commands::Command::parse(&[]).unwrap().name, "help");
    assert!(commands::Command::parse(&["unknown"]).is_none());
    for (args, expected) in [
        (vec!["pause"], Some(1)),
        (vec!["pause", "-d", "indefinitely"], Some(1)),
        (vec!["pause", "-d", "until-morning"], Some(500)),
        (vec!["pause", "-d", "10m"], Some(600_000)),
        (vec!["pause", "-d", "invalid"], None),
    ] {
        assert_eq!(
            commands::Command::parse(&args).unwrap().pause_duration(500),
            expected
        );
    }
}
