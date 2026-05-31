//! Best-effort operator-error guard.
//!
//! IMPORTANT: this is NOT a security boundary and must never be described as
//! one. It cannot stop a determined or obfuscated command, and it is trivially
//! bypassed. Its only job is to make a human pause before an obviously
//! consequential action (wrong env, wrong target) — the same reason a "are you
//! sure?" dialog exists. Treat false negatives as expected.

use serde::Serialize;

/// Substrings that escalate a run to the more deliberate confirmation. These
/// are matched case-insensitively against the fully resolved command string.
const DESTRUCTIVE_TOKENS: &[&str] = &[
    "rm -rf",
    "rm ",
    "kubectl delete",
    "terraform apply",
    "terraform destroy",
    "drop ",
    "--force",
    "push --force",
];

#[derive(Debug, Clone, Serialize)]
pub struct SafetyAssessment {
    /// True when at least one destructive token matched.
    pub escalate: bool,
    /// The specific tokens that matched, for display in the confirmation UI.
    pub matched: Vec<String>,
}

/// Assess a fully resolved command string. Case-insensitive substring match.
pub fn assess(resolved_command: &str) -> SafetyAssessment {
    let haystack = resolved_command.to_lowercase();
    let matched: Vec<String> = DESTRUCTIVE_TOKENS
        .iter()
        .filter(|tok| haystack.contains(&tok.to_lowercase()))
        .map(|tok| tok.trim().to_string())
        .collect();

    SafetyAssessment {
        escalate: !matched.is_empty(),
        matched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_destructive_tokens() {
        assert!(assess("rm -rf build").escalate);
        assert!(assess("kubectl delete pod x").escalate);
        assert!(assess("terraform apply -auto-approve").escalate);
        assert!(assess("git push --force origin main").escalate);
    }

    #[test]
    fn leaves_ordinary_commands_alone() {
        assert!(!assess("npm run build").escalate);
        assert!(!assess("make test").escalate);
        assert!(!assess("cargo check").escalate);
    }

    #[test]
    fn match_is_case_insensitive() {
        assert!(assess("Terraform Destroy").escalate);
    }
}
