use codespan_reporting::diagnostic::{Diagnostic as CodespanDiagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use tokr_span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct LintFix {
    pub span: Span,
    pub replacement: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
    pub fix: Option<LintFix>,
}

#[derive(Default, Debug, Clone)]
pub struct DiagnosticBag(Vec<Diagnostic>);

impl DiagnosticBag {
    pub fn error(&mut self, code: &'static str, msg: impl Into<String>, span: Span) {
        self.0.push(Diagnostic {
            severity: Severity::Error,
            code,
            message: msg.into(),
            span,
            notes: Vec::new(),
            fix: None,
        });
    }

    pub fn warn(&mut self, code: &'static str, msg: impl Into<String>, span: Span) {
        self.0.push(Diagnostic {
            severity: Severity::Warning,
            code,
            message: msg.into(),
            span,
            notes: Vec::new(),
            fix: None,
        });
    }

    pub fn warn_with_fix(
        &mut self,
        code: &'static str,
        msg: impl Into<String>,
        span: Span,
        fix: LintFix,
    ) {
        self.0.push(Diagnostic {
            severity: Severity::Warning,
            code,
            message: msg.into(),
            span,
            notes: Vec::new(),
            fix: Some(fix),
        });
    }

    pub fn has_errors(&self) -> bool {
        self.0.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn promote_warnings_to_errors(&mut self) {
        for diag in &mut self.0 {
            if diag.severity == Severity::Warning {
                diag.severity = Severity::Error;
            }
        }
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.0
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter()
    }

    pub fn extend(&mut self, other: impl IntoIterator<Item = Diagnostic>) {
        self.0.extend(other)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[allow(deprecated)]
pub fn render_diagnostics(file_name: &str, src: &str, diags: &[Diagnostic]) {
    let mut files = SimpleFiles::new();
    let file_id = files.add(file_name, src);

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();

    for d in diags {
        let severity = match d.severity {
            Severity::Error => codespan_reporting::diagnostic::Severity::Error,
            Severity::Warning => codespan_reporting::diagnostic::Severity::Warning,
        };

        let label = Label::primary(file_id, (d.span.lo as usize)..(d.span.hi as usize));

        let diagnostic = CodespanDiagnostic::new(severity)
            .with_code(d.code)
            .with_message(&d.message)
            .with_labels(vec![label])
            .with_notes(if let Some(fix) = &d.fix {
                let mut n = d.notes.clone();
                n.push(format!("Suggestion: replace with `{}`", fix.replacement));
                n
            } else {
                d.notes.clone()
            });

        let _ = term::emit(&mut writer.lock(), &config, &files, &diagnostic);
    }
}
