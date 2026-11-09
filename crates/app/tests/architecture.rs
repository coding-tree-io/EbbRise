#[test]
fn domain_dependencies_follow_module_boundaries() {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let workspace = metadata.workspace_packages();
    let allowed: &[(&str, &[&str])] = &[
        ("settings", &[]),
        ("breaks", &["settings"]),
        ("commands", &[]),
    ];

    for (name, edges) in allowed {
        let package = workspace
            .iter()
            .find(|package| package.name == *name)
            .unwrap();
        let forbidden = package.dependencies.iter().find(|dependency| {
            let name = dependency.name.as_str();
            (workspace.iter().any(|package| package.name == name)
                || ["windows", "platform", "ui", "app"].contains(&name))
                && !edges.contains(&name)
        });
        assert!(
            forbidden.is_none(),
            "{} may not depend on {forbidden:?}",
            package.name
        );
    }
}
