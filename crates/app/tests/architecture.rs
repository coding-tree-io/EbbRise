//! Workspace rules: module dependencies and source file size (ADR 0002).

use std::path::{Path, PathBuf};

/// Strict end of the 300–500 lines that Rust community tools use (rustqual, dictator-rust).
const MAX_CODE_LINES_PER_FILE: usize = 300;

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn code_lines(source: &str) -> usize {
    source
        .lines()
        .map(str::trim)
        .take_while(|line| *line != "#[cfg(test)]")
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .count()
}

#[test]
fn source_files_stay_within_the_code_line_limit() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for crate_dir in std::fs::read_dir(crates).unwrap() {
        let source = crate_dir.unwrap().path().join("src");
        if source.is_dir() {
            rust_files(&source, &mut files);
        }
    }
    assert!(!files.is_empty());
    let too_long: Vec<_> = files
        .iter()
        .map(|file| (file, code_lines(&std::fs::read_to_string(file).unwrap())))
        .filter(|(_, lines)| *lines > MAX_CODE_LINES_PER_FILE)
        .collect();
    assert!(
        too_long.is_empty(),
        "over {MAX_CODE_LINES_PER_FILE} code lines: {too_long:?}"
    );
}

#[test]
fn domain_dependencies_follow_module_boundaries() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .unwrap();
    let workspace = metadata.workspace_packages();
    let allowed: &[(&str, &[&str])] = &[
        ("settings", &[]),
        ("breaks", &["settings"]),
        ("commands", &[]),
        ("i18n", &[]),
        ("ideas", &["settings"]),
        ("platform", &["windows"]),
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
