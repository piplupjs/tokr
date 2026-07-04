use tokr_ast::{File, Value};
use tokr_config::HexCase;
use tokr_diagnostics::{DiagnosticBag, LintFix};
use tokr_linter::LintRule;

pub struct HexCaseRule {
    case: HexCase,
}

impl HexCaseRule {
    pub fn new(case: HexCase) -> Self {
        Self { case }
    }
}

impl LintRule for HexCaseRule {
    fn name(&self) -> &'static str {
        "hex-case"
    }

    fn check_file(&self, file: &File, diags: &mut DiagnosticBag) {
        for decl in &file.decls {
            if let Value::Raw { text, span } = &decl.value {
                if text.starts_with('#') {
                    let expected = match self.case {
                        HexCase::Lower => text.to_lowercase(),
                        HexCase::Upper => text.to_uppercase(),
                    };

                    if text.as_str() != expected {
                        let fix = LintFix {
                            span: *span,
                            replacement: expected.clone(),
                        };
                        diags.warn_with_fix(
                            "LINT002",
                            format!("Hex color '{}' should be {:?}", text, self.case),
                            *span,
                            fix,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;
    use tokr_ast::{Path, ThemeDecl};
    use tokr_span::Span;

    fn test_file(var_name: &str, is_sass_var: bool, val: Value) -> File {
        File {
            decls: vec![ThemeDecl {
                path: Path::new(),
                path_span: Span::new(0, 0),
                var_name: SmolStr::new(var_name),
                is_sass_var,
                value: val,
                span: Span::new(0, 10),
            }],
        }
    }

    #[test]
    fn test_hex_case() {
        let rule = HexCaseRule::new(HexCase::Lower);
        let mut diags = DiagnosticBag::default();

        // Valid
        rule.check_file(
            &test_file(
                "a",
                true,
                Value::Raw {
                    text: "#ffffff".into(),
                    span: Span::new(0, 0),
                },
            ),
            &mut diags,
        );
        assert!(diags.is_empty());

        // Invalid
        rule.check_file(
            &test_file(
                "a",
                true,
                Value::Raw {
                    text: "#FFFFFF".into(),
                    span: Span::new(0, 0),
                },
            ),
            &mut diags,
        );
        assert!(!diags.is_empty());
        let diag = diags.into_vec().into_iter().next().unwrap();
        assert_eq!(diag.fix.unwrap().replacement, "#ffffff");
    }
}
