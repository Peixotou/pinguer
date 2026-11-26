//! Compiler diagnostics for Pinguer.
//!
//! A [`Diagnostic`] describes a single problem found in the program: its
//! [`Severity`], an optional stable [`Code`], a message, labelled source
//! [`Span`]s and any notes or suggestions. Diagnostics are plain data; they are
//! turned into text by the renderers in [`human`] and [`json`].
//!
//! Every compiler stage reports problems into a [`Diagnostics`] collection
//! instead of stopping at the first error, so that a single run can report as
//! many independent problems as possible. A collection can be given a limit
//! on the diagnostics it retains; further diagnostics are counted but not
//! stored, and [`Diagnostics::suppression_notice`] says how many there were.

pub mod codes;
pub mod human;
pub mod json;

use std::fmt;

pub use codes::Code;
use pinguer_source::Span;

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// The program is invalid; compilation cannot produce an artifact.
    Error,
    /// The program is valid but probably not what the author intended.
    Warning,
    /// Additional information that is not a problem by itself.
    Note,
}

impl Severity {
    /// Returns the lowercase name used in rendered output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a label marks the main location of a diagnostic or related context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LabelStyle {
    /// The location where the problem was detected. Rendered with `^`.
    Primary,
    /// A related location that helps explain the problem. Rendered with `-`.
    Secondary,
}

/// A message attached to a span of source code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// The source range being labelled.
    pub span: Span,
    /// Text displayed next to the underlined range. May be empty.
    pub message: String,
    /// Primary or secondary.
    pub style: LabelStyle,
}

/// A single problem or remark reported by the compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// How serious the diagnostic is.
    pub severity: Severity,
    /// Stable identifier, if this diagnostic has one.
    pub code: Option<Code>,
    /// One-line summary of the problem, in lowercase without a trailing period.
    pub message: String,
    /// Labelled source locations. The first primary label determines the
    /// location reported in the diagnostic header.
    pub labels: Vec<Label>,
    /// Additional context rendered as `= note: ...`.
    pub notes: Vec<String>,
    /// Suggestions rendered as `= help: ...`.
    ///
    /// A help message must only propose a change when the compiler has the
    /// semantic information to know the change is valid.
    pub help: Vec<String>,
}

impl Diagnostic {
    /// Creates a diagnostic with the given severity and message.
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: Vec::new(),
        }
    }

    /// Creates an error diagnostic.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    /// Creates a warning diagnostic.
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    /// Sets the diagnostic code.
    #[must_use]
    pub fn with_code(mut self, code: Code) -> Self {
        self.code = Some(code);
        self
    }

    /// Adds a primary label.
    #[must_use]
    pub fn with_primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        });
        self
    }

    /// Adds a secondary label.
    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        });
        self
    }

    /// Adds a note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Adds a help message.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help.push(help.into());
        self
    }

    /// Returns the label that determines the reported location: the first
    /// primary label, or the first label if there is no primary one.
    #[must_use]
    pub fn main_label(&self) -> Option<&Label> {
        self.labels
            .iter()
            .find(|label| label.style == LabelStyle::Primary)
            .or_else(|| self.labels.first())
    }
}

/// An ordered collection of diagnostics produced during a compilation.
///
/// A collection created with [`Diagnostics::with_limit`] retains at most that
/// many diagnostics: the first ones reported. Later diagnostics are counted
/// but not stored, so [`Diagnostics::error_count`] and
/// [`Diagnostics::has_errors`] always describe everything that was reported,
/// while [`Diagnostics::len`], [`Diagnostics::iter`] and the other accessors
/// see only the retained diagnostics.
#[derive(Clone, Debug, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
    /// The maximum number of retained diagnostics, or `None` for no limit.
    limit: Option<usize>,
    errors: usize,
    warnings: usize,
    suppressed: usize,
}

impl Diagnostics {
    /// Creates an empty collection without a limit.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty collection that retains at most `limit` diagnostics.
    ///
    /// # Panics
    ///
    /// Panics if `limit` is zero.
    #[must_use]
    pub fn with_limit(limit: usize) -> Self {
        assert!(limit > 0, "a diagnostic limit must be at least one");
        Self {
            limit: Some(limit),
            ..Self::default()
        }
    }

    /// Returns the maximum number of retained diagnostics, if there is one.
    #[must_use]
    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    /// Returns how many more diagnostics will be retained, or `None` if the
    /// collection has no limit.
    #[must_use]
    pub fn remaining_capacity(&self) -> Option<usize> {
        self.limit.map(|limit| limit - self.items.len())
    }

    /// Records a diagnostic. If the limit has been reached, the diagnostic is
    /// counted and dropped.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.count(diagnostic.severity, 1);
        if self.remaining_capacity() == Some(0) {
            self.suppressed += 1;
        } else {
            self.items.push(diagnostic);
        }
    }

    /// Counts `count` diagnostics of `severity` that their producer
    /// discarded without pushing them, because they would have been
    /// suppressed anyway. Keeps the counts exact for stages that bound the
    /// memory they spend on diagnostics.
    pub fn record_suppressed(&mut self, severity: Severity, count: usize) {
        self.count(severity, count);
        self.suppressed += count;
    }

    fn count(&mut self, severity: Severity, count: usize) {
        match severity {
            Severity::Error => self.errors += count,
            Severity::Warning => self.warnings += count,
            Severity::Note => {}
        }
    }

    /// Returns `true` if at least one error has been reported, including
    /// suppressed errors.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    /// Returns the number of reported errors, including suppressed errors.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.errors
    }

    /// Returns the number of reported warnings, including suppressed
    /// warnings.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.warnings
    }

    /// Returns the number of diagnostics that were reported but not retained.
    #[must_use]
    pub fn suppressed_count(&self) -> usize {
        self.suppressed
    }

    /// Returns a note stating how many diagnostics were suppressed, or `None`
    /// if none were. Renderers print it after the retained diagnostics so
    /// that reaching the limit is never silent.
    #[must_use]
    pub fn suppression_notice(&self) -> Option<Diagnostic> {
        let limit = self.limit?;
        if self.suppressed == 0 {
            return None;
        }
        let plural = if self.suppressed == 1 { "" } else { "s" };
        Some(
            Diagnostic::new(
                Severity::Note,
                format!("{} more diagnostic{plural} not shown", self.suppressed),
            )
            .with_code(codes::DIAGNOSTICS_SUPPRESSED)
            .with_note(format!(
                "at most {limit} diagnostics are reported in one compilation"
            )),
        )
    }

    /// Returns `true` if no diagnostic has been retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of retained diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns the retained diagnostics in the order they were reported.
    #[must_use]
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.items
    }

    /// Returns an iterator over the retained diagnostics.
    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.items.iter()
    }

    /// Consumes the collection and returns the retained diagnostics.
    #[must_use]
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.items
    }
}

impl<'a> IntoIterator for &'a Diagnostics {
    type Item = &'a Diagnostic;
    type IntoIter = std::slice::Iter<'a, Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinguer_source::SourceMap;

    #[test]
    fn counts_errors_and_warnings() {
        let mut diagnostics = Diagnostics::new();
        assert!(!diagnostics.has_errors());
        diagnostics.push(Diagnostic::warning("w"));
        diagnostics.push(Diagnostic::error("e1"));
        diagnostics.push(Diagnostic::error("e2"));
        assert!(diagnostics.has_errors());
        assert_eq!(diagnostics.error_count(), 2);
        assert_eq!(diagnostics.warning_count(), 1);
        assert_eq!(diagnostics.len(), 3);
    }

    #[test]
    fn limit_retains_the_first_diagnostics_and_counts_the_rest() {
        let mut diagnostics = Diagnostics::with_limit(2);
        assert_eq!(diagnostics.remaining_capacity(), Some(2));
        diagnostics.push(Diagnostic::error("e1"));
        diagnostics.push(Diagnostic::warning("w1"));
        diagnostics.push(Diagnostic::error("e2"));
        diagnostics.push(Diagnostic::warning("w2"));
        diagnostics.push(Diagnostic::error("e3"));
        let retained: Vec<&str> = diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(retained, ["e1", "w1"]);
        assert_eq!(diagnostics.remaining_capacity(), Some(0));
        assert_eq!(diagnostics.error_count(), 3);
        assert_eq!(diagnostics.warning_count(), 2);
        assert_eq!(diagnostics.suppressed_count(), 3);

        let notice = diagnostics.suppression_notice().unwrap();
        assert_eq!(notice.severity, Severity::Note);
        assert_eq!(notice.code, Some(codes::DIAGNOSTICS_SUPPRESSED));
        assert_eq!(notice.message, "3 more diagnostics not shown");
        assert_eq!(
            notice.notes,
            ["at most 2 diagnostics are reported in one compilation"]
        );
    }

    #[test]
    fn no_notice_without_suppression() {
        let mut limited = Diagnostics::with_limit(1);
        limited.push(Diagnostic::error("only"));
        assert!(limited.suppression_notice().is_none());

        let mut unlimited = Diagnostics::new();
        for _ in 0..1_000 {
            unlimited.push(Diagnostic::error("e"));
        }
        assert_eq!(unlimited.len(), 1_000);
        assert_eq!(unlimited.remaining_capacity(), None);
        assert!(unlimited.suppression_notice().is_none());
    }

    #[test]
    fn record_suppressed_keeps_counts_exact() {
        let mut diagnostics = Diagnostics::with_limit(1);
        diagnostics.push(Diagnostic::error("kept"));
        diagnostics.record_suppressed(Severity::Error, 41);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics.error_count(), 42);
        assert_eq!(
            diagnostics.suppression_notice().unwrap().message,
            "41 more diagnostics not shown"
        );
        diagnostics.record_suppressed(Severity::Note, 1);
        assert_eq!(diagnostics.error_count(), 42);
        assert_eq!(diagnostics.suppressed_count(), 42);
    }

    #[test]
    #[should_panic(expected = "at least one")]
    fn zero_limit_is_rejected() {
        let _ = Diagnostics::with_limit(0);
    }

    #[test]
    fn main_label_prefers_primary() {
        let mut map = SourceMap::new();
        let file = map.add("a.pgr", "abcdef").unwrap();
        let secondary = Span::new(file, 0, 1);
        let primary = Span::new(file, 2, 3);
        let diagnostic = Diagnostic::error("x")
            .with_secondary(secondary, "")
            .with_primary(primary, "");
        assert_eq!(diagnostic.main_label().unwrap().span, primary);

        let only_secondary = Diagnostic::error("x").with_secondary(secondary, "");
        assert_eq!(only_secondary.main_label().unwrap().span, secondary);
        assert!(Diagnostic::error("x").main_label().is_none());
    }
}
