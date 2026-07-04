use tokr_ast::File;
use tokr_diagnostics::DiagnosticBag;

pub trait LintRule {
    fn name(&self) -> &'static str;
    fn check_file(&self, file: &File, diags: &mut DiagnosticBag);
}

pub struct Linter {
    rules: Vec<Box<dyn LintRule>>,
}

impl Linter {
    pub fn new(rules: Vec<Box<dyn LintRule>>) -> Self {
        Self { rules }
    }

    pub fn lint_file(&self, file: &File, diags: &mut DiagnosticBag) {
        for rule in &self.rules {
            rule.check_file(file, diags);
        }
    }
}
