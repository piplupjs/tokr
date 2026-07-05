use indexmap::IndexMap;
use tokr_ast::{PathSegment, Value};
use tokr_sema::SymbolTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrValue {
    CssVarRef(String), // renders as "var(--foo)"
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrNode {
    Object(IndexMap<String, IrNode>),
    Array(Vec<IrNode>),
    Leaf(IrValue),
    Hole,
}

impl IrNode {
    pub fn as_object_mut(&mut self) -> &mut IndexMap<String, IrNode> {
        match self {
            IrNode::Object(map) => map,
            _ => panic!("Expected Object"),
        }
    }

    pub fn as_array_mut(&mut self) -> &mut Vec<IrNode> {
        match self {
            IrNode::Array(arr) => arr,
            _ => panic!("Expected Array"),
        }
    }
}

pub fn lower(symbols: &SymbolTable) -> IrNode {
    let mut root = IrNode::Object(IndexMap::new());

    for (path, symbol) in &symbols.entries {
        let value = match &symbol.value {
            Value::VarRef { css_var, .. } => IrValue::CssVarRef(css_var.to_string()),
            Value::Raw { text, .. } => IrValue::Raw(text.to_string()),
        };
        insert(&mut root, path, value);
    }

    root
}

fn insert(root: &mut IrNode, path: &[PathSegment], value: IrValue) {
    match path {
        [] => unreachable!("empty path rejected in sema"),
        [PathSegment::Field(name)] => {
            let obj = root.as_object_mut(); // panics-as-bug if shape conflict slipped past sema
            obj.insert(name.to_string(), IrNode::Leaf(value));
        }
        [PathSegment::Field(name), rest @ ..] => {
            let obj = root.as_object_mut();
            let child = obj
                .entry(name.to_string())
                .or_insert_with(|| default_container(rest));
            insert(child, rest, value);
        }
        [PathSegment::Index(i)] => {
            let arr = root.as_array_mut();
            ensure_len(arr, *i as usize + 1); // grows with IrNode::Hole placeholders
            arr[*i as usize] = IrNode::Leaf(value);
        }
        [PathSegment::Index(i), rest @ ..] => {
            let arr = root.as_array_mut();
            ensure_len(arr, *i as usize + 1);
            if matches!(arr[*i as usize], IrNode::Hole) {
                arr[*i as usize] = default_container(rest);
            }
            insert(&mut arr[*i as usize], rest, value);
        }
    }
}

fn default_container(rest: &[PathSegment]) -> IrNode {
    match rest.first() {
        Some(PathSegment::Field(_)) => IrNode::Object(IndexMap::new()),
        Some(PathSegment::Index(_)) => IrNode::Array(Vec::new()),
        None => unreachable!(),
    }
}

fn ensure_len(arr: &mut Vec<IrNode>, len: usize) {
    if arr.len() < len {
        arr.resize(len, IrNode::Hole);
    }
}

use std::collections::HashMap;

#[derive(Default)]
pub struct PassConfig {
    pub strict: bool,
    pub order_table: HashMap<String, Vec<String>>, // key = dotted prefix, "" = root
}

pub fn run_passes(ir: IrNode, cfg: &PassConfig) -> IrNode {
    let ir = fill_or_reject_holes(ir, cfg);
    ordering_pass(ir, cfg, "")
}

fn ordering_pass(node: IrNode, cfg: &PassConfig, current_path: &str) -> IrNode {
    match node {
        IrNode::Object(mut map) => {
            // First recurse into children
            for (k, v) in map.iter_mut() {
                let child_path = if current_path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", current_path, k)
                };
                let old = std::mem::replace(v, IrNode::Hole);
                *v = ordering_pass(old, cfg, &child_path);
            }

            // Then sort this map
            let order = cfg.order_table.get(current_path);
            map.sort_by(|k1, _, k2, _| {
                if let Some(order) = order {
                    let pos1 = order.iter().position(|x| x == k1);
                    let pos2 = order.iter().position(|x| x == k2);
                    match (pos1, pos2) {
                        (Some(p1), Some(p2)) => p1.cmp(&p2),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => k1.cmp(k2),
                    }
                } else {
                    k1.cmp(k2)
                }
            });

            IrNode::Object(map)
        }
        IrNode::Array(mut arr) => {
            for (i, v) in arr.iter_mut().enumerate() {
                let child_path = format!("{}[{}]", current_path, i);
                let old = std::mem::replace(v, IrNode::Hole);
                *v = ordering_pass(old, cfg, &child_path);
            }
            IrNode::Array(arr)
        }
        IrNode::Leaf(v) => IrNode::Leaf(v),
        IrNode::Hole => unreachable!(),
    }
}

fn fill_or_reject_holes(node: IrNode, cfg: &PassConfig) -> IrNode {
    match node {
        IrNode::Object(mut map) => {
            for (_, v) in map.iter_mut() {
                let old = std::mem::replace(v, IrNode::Hole);
                *v = fill_or_reject_holes(old, cfg);
            }
            IrNode::Object(map)
        }
        IrNode::Array(mut arr) => {
            for v in arr.iter_mut() {
                if matches!(v, IrNode::Hole) {
                    if cfg.strict {
                        panic!("Holes are not allowed in strict mode");
                    } else {
                        *v = IrNode::Leaf(IrValue::Raw("undefined".into()));
                    }
                } else {
                    let old = std::mem::replace(v, IrNode::Hole);
                    *v = fill_or_reject_holes(old, cfg);
                }
            }
            IrNode::Array(arr)
        }
        IrNode::Leaf(v) => IrNode::Leaf(v),
        IrNode::Hole => {
            if cfg.strict {
                panic!("Holes are not allowed in strict mode");
            } else {
                IrNode::Leaf(IrValue::Raw("undefined".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;
    use tokr_ast::{File, ThemeDecl, Value};
    use tokr_span::Span;

    fn test_table(decls: Vec<(&'static str, Value)>) -> SymbolTable {
        let mut file = File::default();
        for (path_str, val) in decls {
            let mut path = Vec::new();
            for seg in path_str.split('.') {
                if seg.ends_with(']') {
                    let parts: Vec<&str> = seg.split('[').collect();
                    if !parts[0].is_empty() {
                        path.push(PathSegment::Field(SmolStr::new(parts[0])));
                    }
                    let idx = parts[1].trim_end_matches(']').parse::<u32>().unwrap();
                    path.push(PathSegment::Index(idx));
                } else {
                    path.push(PathSegment::Field(SmolStr::new(seg)));
                }
            }
            file.decls.push(ThemeDecl {
                path,
                path_span: Span::new(0, 0),
                var_name: SmolStr::new(""),
                is_sass_var: true,
                value: val,
                span: Span::new(0, 0),
            });
        }
        let mut bag = tokr_diagnostics::DiagnosticBag::default();
        tokr_sema::analyze(&file, &mut bag)
    }

    #[test]
    fn test_lower_object() {
        let table = test_table(vec![
            (
                "palette.accent",
                Value::Raw {
                    text: SmolStr::new("1"),
                    span: Span::new(0, 0),
                },
            ),
            (
                "palette.primary",
                Value::Raw {
                    text: SmolStr::new("2"),
                    span: Span::new(0, 0),
                },
            ),
        ]);
        let ir = lower(&table);
        let cfg = PassConfig {
            strict: false,
            order_table: HashMap::new(),
        };
        let ir = run_passes(ir, &cfg);

        match ir {
            IrNode::Object(map) => {
                let palette = map.get("palette").unwrap();
                match palette {
                    IrNode::Object(p_map) => {
                        assert!(p_map.contains_key("accent"));
                        assert!(p_map.contains_key("primary"));
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_lower_array() {
        let table = test_table(vec![
            (
                "boxShadow[0]",
                Value::Raw {
                    text: SmolStr::new("0"),
                    span: Span::new(0, 0),
                },
            ),
            (
                "boxShadow[2]",
                Value::Raw {
                    text: SmolStr::new("2"),
                    span: Span::new(0, 0),
                },
            ),
        ]);
        let ir = lower(&table);
        let cfg = PassConfig {
            strict: false,
            order_table: HashMap::new(),
        };
        let ir = run_passes(ir, &cfg);

        match ir {
            IrNode::Object(map) => {
                let arr = map.get("boxShadow").unwrap();
                match arr {
                    IrNode::Array(a) => {
                        assert_eq!(a.len(), 3);
                        assert_eq!(a[0], IrNode::Leaf(IrValue::Raw("0".into())));
                        assert_eq!(a[1], IrNode::Leaf(IrValue::Raw("undefined".into())));
                        assert_eq!(a[2], IrNode::Leaf(IrValue::Raw("2".into())));
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_ordering_pass() {
        let table = test_table(vec![
            (
                "palette.secondary",
                Value::Raw {
                    text: SmolStr::new("2"),
                    span: Span::new(0, 0),
                },
            ),
            (
                "palette.primary",
                Value::Raw {
                    text: SmolStr::new("1"),
                    span: Span::new(0, 0),
                },
            ),
            (
                "palette.accent",
                Value::Raw {
                    text: SmolStr::new("3"),
                    span: Span::new(0, 0),
                },
            ),
        ]);
        let ir = lower(&table);

        let mut order_table = HashMap::new();
        order_table.insert(
            "palette".to_string(),
            vec![
                "primary".to_string(),
                "secondary".to_string(),
                "accent".to_string(),
            ],
        );

        let cfg = PassConfig {
            strict: false,
            order_table,
        };
        let ir = run_passes(ir, &cfg);

        match ir {
            IrNode::Object(map) => {
                let palette = map.get("palette").unwrap();
                match palette {
                    IrNode::Object(p_map) => {
                        let keys: Vec<_> = p_map.keys().collect();
                        assert_eq!(keys, vec!["primary", "secondary", "accent"]);
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }
}
