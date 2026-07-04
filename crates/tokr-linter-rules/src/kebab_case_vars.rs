use tokr_ast::File;
use tokr_diagnostics::DiagnosticBag;
use tokr_linter::LintRule;

pub struct KebabCaseVarsRule;

impl LintRule for KebabCaseVarsRule {
    fn name(&self) -> &'static str {
        "kebab-case-vars"
    }

    fn check_file(&self, file: &File, diags: &mut DiagnosticBag) {
        for decl in &file.decls {
            if !decl
                .var_name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                let suggested = decl.var_name.replace('_', "-").to_lowercase();

                diags.warn(
                    "LINT001",
                    format!(
                        "Variable name '{}' is not kebab-case (try '{}')",
                        decl.var_name, suggested
                    ),
                    decl.span, // using the full decl span for now
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;
    use tokr_ast::{Path, ThemeDecl, Value};
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
    fn test_kebab_case() {
        let rule = KebabCaseVarsRule;
        let mut diags = DiagnosticBag::default();

        // Valid
        rule.check_file(
            &test_file(
                "my-var",
                true,
                Value::Raw {
                    text: "1".into(),
                    span: Span::new(0, 0),
                },
            ),
            &mut diags,
        );
        assert!(diags.is_empty());

        // Invalid
        rule.check_file(
            &test_file(
                "my_var",
                true,
                Value::Raw {
                    text: "1".into(),
                    span: Span::new(0, 0),
                },
            ),
            &mut diags,
        );
        assert!(!diags.is_empty());
    }
}
