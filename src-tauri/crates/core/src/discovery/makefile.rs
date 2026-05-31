//! Parser for `Makefile` targets.
//!
//! HEURISTIC, on purpose. A real Makefile can do far more than this regex
//! understands (includes, conditionals, computed target names, ...). For v1 we
//! only want the common case: literal target names declared at column 0. We:
//!
//!   * match lines like `^name:` where name starts alphanumeric,
//!   * skip `.PHONY` (and other dot-directives) since those declare, not define,
//!   * skip pattern rules (targets containing `%`),
//!   * capture a same-line `## comment` as the description if present.
//!
//! Invocation is `make <target>`. Anything fancier is out of scope for v1.

use super::{DiscoveredCommand, SourceType};
use once_cell::sync::Lazy;
use regex::Regex;

// Target line: an unindented name made of [A-Za-z0-9_.-], starting with an
// alphanumeric, followed by ':' (but not ':=', which is a variable assignment).
static TARGET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([a-zA-Z0-9][a-zA-Z0-9_.-]*)\s*:(?:[^=]|$)").unwrap()
});

// Optional self-documenting comment: `target: deps ## description`.
static DOC_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"##\s*(.+?)\s*$").unwrap());

pub fn parse(text: &str, repo: &str, cwd: &str) -> Vec<DiscoveredCommand> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in text.lines() {
        let caps = match TARGET_RE.captures(line) {
            Some(c) => c,
            None => continue,
        };
        let name = caps.get(1).unwrap().as_str();

        // Skip dot-directives (.PHONY, .DEFAULT, .SUFFIXES, ...) — they declare
        // metadata, they are not runnable targets.
        if name.starts_with('.') {
            continue;
        }
        // Skip pattern rules.
        if name.contains('%') {
            continue;
        }
        // First declaration wins; a target can legitimately appear twice.
        if !seen.insert(name.to_string()) {
            continue;
        }

        let description = DOC_RE
            .captures(line)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        out.push(DiscoveredCommand::new(
            repo.to_string(),
            SourceType::Make,
            name.to_string(),
            description,
            format!("make {}", name),
            cwd.to_string(),
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
.PHONY: build test clean

# A plain comment, not a target.
build: ## Compile the project
\tcargo build --release

test:
\tcargo test

deploy-prod: build ## Ship it
\t./deploy.sh

%.o: %.c
\t$(CC) -c $<

VERSION := 1.2.3

clean:
\trm -rf target
";

    #[test]
    fn extracts_real_targets_only() {
        let cmds = parse(FIXTURE, "/repo", "/repo");
        let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();

        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"deploy-prod"));
        assert!(names.contains(&"clean"));

        // Excluded: .PHONY directive, the %.o pattern rule, the VERSION
        // variable assignment.
        assert!(!names.contains(&".PHONY"));
        assert!(!names.iter().any(|n| n.contains('%')));
        assert!(!names.contains(&"VERSION"));
        assert_eq!(cmds.len(), 4);
    }

    #[test]
    fn captures_doc_comments_and_invocation() {
        let cmds = parse(FIXTURE, "/repo", "/repo");
        let build = cmds.iter().find(|c| c.name == "build").unwrap();
        assert_eq!(build.invocation, "make build");
        assert_eq!(build.description.as_deref(), Some("Compile the project"));
        assert_eq!(build.source, SourceType::Make);

        // A target with no `##` comment has no description.
        let test = cmds.iter().find(|c| c.name == "test").unwrap();
        assert_eq!(test.description, None);
    }

    #[test]
    fn variable_assignment_is_not_a_target() {
        let cmds = parse("CC := gcc\nCFLAGS = -O2\n", "/r", "/r");
        assert!(cmds.is_empty());
    }
}
