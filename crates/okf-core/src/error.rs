//! Error type and its mapping to CLI exit classes.
use std::sync::Arc;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, OkfError>;

/// Machine-readable classification for failures that callers must tell apart
/// without parsing messages (JSON failure `reasons`, recovery hints).
///
/// The `Display` form is the stable message prefix (`write-gate-incomplete`, …).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureReason {
    /// The write gate could not examine everything it needed (exit 1).
    WriteGateIncomplete,
    /// The write gate found new or worsened ontology findings (exit 1).
    WriteGateRejected,
    /// The write gate could not compare before/after values unambiguously (exit 1).
    WriteGateComparisonAmbiguous,
    /// The previewed baseline no longer matches the bundle (exit 2).
    BaselineConflict,
    /// A bundle-context token or receipt is unusable for this request (exit 2).
    ContextInvalid,
}

impl FailureReason {
    /// The stable message prefix for this reason.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WriteGateIncomplete => "write-gate-incomplete",
            Self::WriteGateRejected => "write-gate-rejected",
            Self::WriteGateComparisonAmbiguous => "write-gate-comparison-ambiguous",
            Self::BaselineConflict => "baseline-conflict",
            Self::ContextInvalid => "context-invalid",
        }
    }

    const fn is_policy(self) -> bool {
        matches!(
            self,
            Self::WriteGateIncomplete
                | Self::WriteGateRejected
                | Self::WriteGateComparisonAmbiguous
        )
    }

    const fn class(self) -> &'static str {
        if self.is_policy() {
            "policy rejected"
        } else {
            "usage error"
        }
    }
}

impl std::fmt::Display for FailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// All fallible operations in okf-core return this. Findings-driven exit codes
/// (0 vs 1) are decided by the CLI, not here.
#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum OkfError {
    /// An explicitly enabled write policy rejected a prepared mutation (exit 1).
    #[error("policy rejected: {0}")]
    Policy(String),
    /// Bad usage / arguments (exit 2).
    #[error("usage error: {0}")]
    Usage(String),
    /// A policy (exit 1) or usage (exit 2) failure with a typed reason.
    /// Displays as `<class>: <reason>: <message>`.
    #[error("{}: {reason}: {message}", reason.class())]
    Classified {
        reason: FailureReason,
        message: String,
    },
    /// Environment problem — missing/unreadable bundle or ontology (exit 3).
    #[error("environment error: {0}")]
    Environment(String),
    /// I/O failure (exit 3). `source` keeps the underlying error (and its
    /// [`std::io::ErrorKind`]) when one exists.
    #[error("I/O error: {message}")]
    Io {
        message: String,
        #[source]
        source: Option<Arc<std::io::Error>>,
    },
    /// YAML frontmatter/ontology parse failure (exit 3). Library-neutral text.
    #[error("YAML parse error: {0}")]
    Yaml(String),
    /// Internal invariant broken (exit 4).
    #[error("internal error: {0}")]
    Internal(String),
}

impl OkfError {
    /// An I/O failure described only by a message (no underlying `io::Error`).
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
            source: None,
        }
    }

    /// An I/O failure with a custom message that keeps `source` for inspection.
    pub fn io_with(message: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            message: message.into(),
            source: Some(Arc::new(source)),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::Policy(message.into())
    }

    /// A failure whose machine-readable reason is `reason`; `message` excludes
    /// the reason prefix, which `Display` adds.
    pub fn classified(reason: FailureReason, message: impl Into<String>) -> Self {
        Self::Classified {
            reason,
            message: message.into(),
        }
    }

    /// The typed reason, if this failure has one.
    pub fn reason(&self) -> Option<FailureReason> {
        match self {
            Self::Classified { reason, .. } => Some(*reason),
            _ => None,
        }
    }

    /// The kind of the underlying `io::Error`, if this is an I/O failure that kept it.
    pub fn io_kind(&self) -> Option<std::io::ErrorKind> {
        match self {
            Self::Io {
                source: Some(source),
                ..
            } => Some(source.kind()),
            _ => None,
        }
    }

    /// Add the failing file without changing the error's exit class or reason.
    pub fn at_path(self, path: &std::path::Path) -> Self {
        let context = |message| format!("{}: {message}", path.display());
        match self {
            Self::Policy(message) => Self::Policy(context(message)),
            Self::Usage(message) => Self::Usage(context(message)),
            Self::Classified { reason, message } => Self::Classified {
                reason,
                message: context(message),
            },
            Self::Environment(message) => Self::Environment(context(message)),
            Self::Io { message, source } => Self::Io {
                message: context(message),
                source,
            },
            Self::Yaml(message) => Self::Yaml(context(message)),
            Self::Internal(message) => Self::Internal(context(message)),
        }
    }

    /// Combine YAML failures from the files actually examined during a bundle load.
    pub fn yaml_files(messages: Vec<String>) -> Self {
        Self::Yaml(if messages.len() == 1 {
            messages.into_iter().next().unwrap()
        } else {
            format!(
                "{} concept files failed to parse:\n{}",
                messages.len(),
                messages.join("\n")
            )
        })
    }

    /// Exit class documented in knowledge/reference/troubleshooting.md.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Policy(_) => 1,
            Self::Usage(_) => 2,
            Self::Classified { reason, .. } => {
                if reason.is_policy() {
                    1
                } else {
                    2
                }
            }
            Self::Environment(_) | Self::Io { .. } | Self::Yaml(_) => 3,
            Self::Internal(_) => 4,
        }
    }
}

impl From<std::io::Error> for OkfError {
    fn from(e: std::io::Error) -> Self {
        Self::io_with(e.to_string(), e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classified_display_and_exit_match_prefixed_messages() {
        let gate = OkfError::classified(FailureReason::WriteGateIncomplete, "ontology unavailable");
        assert_eq!(
            gate.to_string(),
            "policy rejected: write-gate-incomplete: ontology unavailable"
        );
        assert_eq!(gate.exit_code(), 1);
        let conflict = OkfError::classified(FailureReason::BaselineConflict, "preview again");
        assert_eq!(
            conflict.to_string(),
            "usage error: baseline-conflict: preview again"
        );
        assert_eq!(conflict.exit_code(), 2);
    }

    #[test]
    fn io_keeps_source_kind() {
        let error = OkfError::from(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        assert_eq!(error.io_kind(), Some(std::io::ErrorKind::BrokenPipe));
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(OkfError::io("x").io_kind(), None);
        assert_eq!(
            error.at_path(std::path::Path::new("p")).io_kind(),
            Some(std::io::ErrorKind::BrokenPipe)
        );
    }
}
