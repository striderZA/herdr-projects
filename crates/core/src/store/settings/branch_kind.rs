//! The kind of work a task is (`{type}` of `workspace.branch`): what its title says with a conventional prefix
//! (`fix: …`), else the first word in it that names one (`fix the login`), and `feat` when none does.

/// The conventional kinds of change, in the order the help names them.
pub const KINDS: [&str; 11] = ["feat", "fix", "docs", "test", "refactor", "perf", "build", "ci", "chore", "style", "revert"];

/// The kind a title says: a conventional prefix (`fix:`, `feat(ui):`, `refactor!:`), else its first word that names a
/// kind, else `feat`.
pub fn of(title: &str) -> &'static str {
    let low = title.trim().to_lowercase();
    if let Some(k) = prefix(&low) { return k; }
    low.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty())
        .find_map(|w| KINDS.iter().copied().find(|k| *k == w).or_else(|| word(w)))
        .unwrap_or("feat")
}
/// The kind a conventional prefix names, when the title starts with one.
fn prefix(low: &str) -> Option<&'static str> {
    let (head, _) = low.split_once(':')?;
    let word = head.split('(').next().unwrap_or("").trim().trim_end_matches('!');
    KINDS.iter().copied().find(|k| *k == word)
}
/// The kind a word names, for a title with no prefix.
fn word(w: &str) -> Option<&'static str> {
    Some(match w {
        "fix" | "fixes" | "fixed" | "fixing" | "hotfix" | "bug" | "bugs" | "crash" | "crashes" | "error" | "errors"
        | "broken" | "breakage" | "regression" | "regressions" | "fail" | "fails" | "failing" | "failure" | "failures"
        | "wrong" | "incorrect" | "defect" | "defects" | "glitch" => "fix",
        "refactor" | "refactors" | "refactoring" | "cleanup" | "clean" | "simplify" | "simplifies" | "simplifying"
        | "restructure" | "restructures" | "rewrite" | "rewrites" | "extract" | "extracts" | "rename" | "renames"
        | "dedupe" | "inline" => "refactor",
        "doc" | "docs" | "document" | "documents" | "documenting" | "documentation" | "readme" | "comment" | "comments"
        | "changelog" | "guide" => "docs",
        "test" | "tests" | "testing" | "tested" | "coverage" | "spec" | "specs" => "test",
        "perf" | "performance" | "speed" | "faster" | "slow" | "slower" | "optimize" | "optimise" | "optimization"
        | "optimisation" | "latency" => "perf",
        "ci" | "workflow" | "workflows" | "pipeline" | "pipelines" => "ci",
        "build" | "builds" | "deps" | "dep" | "dependencies" | "dependency" | "upgrade" | "upgrades" | "bump"
        | "bumping" | "toolchain" | "compile" => "build",
        "style" | "styles" | "formatting" | "format" | "formats" | "lint" | "lints" | "clippy" | "whitespace" => "style",
        "chore" | "chores" | "release" | "releases" | "tag" | "tags" | "version" | "versions" | "tidy" => "chore",
        "revert" | "reverts" | "reverted" => "revert",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_prefix_wins_and_a_word_says_it_otherwise() {
        assert_eq!(of("fix: the login"), "fix");
        assert_eq!(of("Feat(ui)!: a panel"), "feat");
        assert_eq!(of("refactor!: one file"), "refactor");
        assert_eq!(of("Fix the login"), "fix");
        assert_eq!(of("The login crashes on save"), "fix");
        assert_eq!(of("Refactor the branch naming"), "refactor");
        assert_eq!(of("Document the settings"), "docs");
        assert_eq!(of("Add tests for the board"), "test");
        assert_eq!(of("Bump the dependencies"), "build");
        assert_eq!(of("Add a greeting"), "feat");
        assert_eq!(of("Look into it"), "feat");
        assert_eq!(of(""), "feat");
        // A colon that names no kind is no prefix: the words still say it.
        assert_eq!(of("Fix the login: an idea"), "fix");
        // A kind inside a longer word says nothing.
        assert_eq!(of("Prefix the log"), "feat");
    }
}
