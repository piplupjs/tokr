use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use tokr_ast::{File, Path, PathSegment, ThemeDecl, Value};
use tokr_diagnostics::DiagnosticBag;
use tokr_span::Span;

pub struct ThemeSymbol {
    pub value: Value,
    pub decl_span: Span,
}

#[derive(Default)]
pub struct SymbolTable {
    pub entries: IndexMap<Path, ThemeSymbol>,
}

pub fn analyze(file: &File, diags: &mut DiagnosticBag) -> SymbolTable {
    let mut table = SymbolTable::default();

    for decl in &file.decls {
        check_name_consistency(decl, diags);
        check_duplicate_paths(&mut table, decl, diags);
        if !table.entries.contains_key(&decl.path) {
            table.entries.insert(
                decl.path.clone(),
                ThemeSymbol {
                    value: decl.value.clone(),
                    decl_span: decl.span,
                },
            );
        }
    }

    check_path_shape_conflicts(&table, diags);
    check_array_index_contiguity(&table, diags);

    table
}

fn check_duplicate_paths(table: &mut SymbolTable, decl: &ThemeDecl, diags: &mut DiagnosticBag) {
    if let Some(existing) = table.entries.get(&decl.path) {
        diags.error("TC0201", "duplicate path", decl.path_span);
        diags.error("TC0201", "previously declared here", existing.decl_span);
    }
}

fn check_name_consistency(decl: &ThemeDecl, diags: &mut DiagnosticBag) {
    if let Value::VarRef { css_var, span } = &decl.value {
        let expected = format!("--{}", decl.var_name);
        if css_var.as_str() != expected {
            let var_type = if decl.is_sass_var {
                "scss variable $"
            } else {
                "css variable --"
            };
            diags.warn(
                "TC0202",
                format!(
                    "name mismatch: {}{} but css variable {}",
                    var_type, decl.var_name, css_var
                ),
                *span,
            );
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum Shape {
    Leaf(Span),
    Object(Span),
    Array(Span),
}

fn check_path_shape_conflicts(table: &SymbolTable, diags: &mut DiagnosticBag) {
    let mut tree_types: HashMap<&[PathSegment], Shape> = HashMap::new();

    for (path, symbol) in &table.entries {
        for i in 0..=path.len() {
            let prefix = &path[0..i];
            let shape = if i == path.len() {
                Shape::Leaf(symbol.decl_span)
            } else {
                match &path[i] {
                    PathSegment::Field(_) => Shape::Object(symbol.decl_span),
                    PathSegment::Index(_) => Shape::Array(symbol.decl_span),
                }
            };

            if let Some(existing) = tree_types.get(prefix) {
                match (existing, &shape) {
                    (Shape::Leaf(_), Shape::Object(_))
                    | (Shape::Object(_), Shape::Leaf(_))
                    | (Shape::Leaf(_), Shape::Array(_))
                    | (Shape::Array(_), Shape::Leaf(_)) => {
                        diags.error(
                            "TC0203",
                            "path is used as both a leaf and a container",
                            symbol.decl_span,
                        );
                    }
                    (Shape::Object(_), Shape::Array(_)) | (Shape::Array(_), Shape::Object(_)) => {
                        diags.error(
                            "TC0203",
                            "path mixes object and array children",
                            symbol.decl_span,
                        );
                    }
                    _ => {}
                }
            } else {
                tree_types.insert(prefix, shape);
            }
        }
    }
}

fn check_array_index_contiguity(table: &SymbolTable, diags: &mut DiagnosticBag) {
    let mut arrays: HashMap<&[PathSegment], HashSet<u32>> = HashMap::new();

    for (path, _) in &table.entries {
        for i in 0..path.len() {
            if let PathSegment::Index(idx) = path[i] {
                let prefix = &path[0..i];
                arrays.entry(prefix).or_default().insert(idx);
            }
        }
    }

    for (prefix, indices) in arrays {
        let max = *indices.iter().max().unwrap();
        for i in 0..=max {
            if !indices.contains(&i) {
                let span = table
                    .entries
                    .iter()
                    .find(|(p, _)| {
                        p.starts_with(prefix)
                            && p.len() > prefix.len()
                            && matches!(p[prefix.len()], PathSegment::Index(_))
                    })
                    .map(|(_, s)| s.decl_span)
                    .unwrap_or(Span::new(0, 0));

                diags.warn("TC0204", format!("sparse array at index {}", i), span);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_file(src: &str) -> (File, DiagnosticBag) {
        let (tokens, lex_diags) = tokr_lexer::Lexer::new(src).tokenize();
        let mut bag = DiagnosticBag::default();
        bag.extend(lex_diags);
        let parser = tokr_parser::Parser::new(src, &tokens, &mut bag);
        (parser.parse_file(), bag)
    }

    fn analyze_src(src: &str) -> (SymbolTable, DiagnosticBag) {
        let (file, mut bag) = parse_test_file(src);
        let table = analyze(&file, &mut bag);
        (table, bag)
    }

    #[test]
    fn test_duplicate_path() {
        let src = "/* @theme a.b */\n$a: 1;\n/* @theme a.b */\n$b: 2;";
        let (_, diags) = analyze_src(src);
        let errs = diags.into_vec();
        assert!(errs.iter().any(|d| d.code == "TC0201"));
    }

    #[test]
    fn test_name_consistency() {
        let src = "/* @theme a.b */\n$accent: var(--primary);";
        let (_, diags) = analyze_src(src);
        let errs = diags.into_vec();
        assert!(errs.iter().any(|d| d.code == "TC0202"));
    }

    #[test]
    fn test_path_shape_conflicts() {
        let src = "/* @theme a.b */\n$a: 1;\n/* @theme a.b.c */\n$b: 2;";
        let (_, diags) = analyze_src(src);
        let errs = diags.into_vec();
        assert!(errs.iter().any(|d| d.code == "TC0203")); // Leaf vs Object

        let src2 = "/* @theme a[0] */\n$a: 1;\n/* @theme a.b */\n$b: 2;";
        let (_, diags2) = analyze_src(src2);
        let errs2 = diags2.into_vec();
        assert!(errs2.iter().any(|d| d.code == "TC0203")); // Array vs Object
    }

    #[test]
    fn test_array_index_contiguity() {
        let src = "/* @theme a[0] */\n$a: 1;\n/* @theme a[2] */\n$b: 2;";
        let (_, diags) = analyze_src(src);
        let errs = diags.into_vec();
        assert!(errs.iter().any(|d| d.code == "TC0204")); // Sparse array
    }
}
