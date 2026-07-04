use smol_str::SmolStr;
use tokr_span::Span;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    Field(SmolStr),
    Index(u32),
}

pub type Path = Vec<PathSegment>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    VarRef { css_var: SmolStr, span: Span },
    Raw { text: SmolStr, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeDecl {
    pub path: Path,
    pub path_span: Span,
    pub var_name: SmolStr,
    pub is_sass_var: bool,
    pub value: Value,
    pub span: Span,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct File {
    pub decls: Vec<ThemeDecl>,
}
