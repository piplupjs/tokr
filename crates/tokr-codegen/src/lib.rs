use std::io::Write;
use std::process::{Command, Stdio};
use tokr_ir::{IrNode, IrValue};

pub fn generate_ts(ir: &IrNode) -> String {
    let mut out = String::new();
    out.push_str("export const theme = ");
    generate_node(ir, &mut out);
    out.push_str(" as const;\n");

    // Format using oxfmt via npx
    format_with_oxfmt(&out)
}

pub fn generate_js(ir: &IrNode) -> String {
    let mut out = String::new();
    out.push_str("export const theme = ");
    generate_node(ir, &mut out);
    out.push_str(";\n");

    format_with_oxfmt(&out)
}

pub fn generate_dts(ir: &IrNode) -> String {
    let mut out = String::new();
    out.push_str("export declare const theme: ");
    generate_type_node(ir, &mut out);
    out.push_str(";\n");

    format_with_oxfmt(&out)
}

fn generate_type_node(node: &IrNode, out: &mut String) {
    match node {
        IrNode::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                out.push_str(&format!("\"{}\": ", k));
                generate_type_node(v, out);
                if i < map.len() - 1 {
                    out.push(',');
                }
            }
            out.push('}');
        }
        IrNode::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                generate_type_node(v, out);
                if i < arr.len() - 1 {
                    out.push(',');
                }
            }
            out.push(']');
        }
        IrNode::Leaf(_) => {
            out.push_str("string");
        }
        IrNode::Hole => {
            out.push_str("undefined");
        }
    }
}

fn generate_node(node: &IrNode, out: &mut String) {
    match node {
        IrNode::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                out.push_str(&format!("\"{}\": ", k));
                generate_node(v, out);
                if i < map.len() - 1 {
                    out.push(',');
                }
            }
            out.push('}');
        }
        IrNode::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                generate_node(v, out);
                if i < arr.len() - 1 {
                    out.push(',');
                }
            }
            out.push(']');
        }
        IrNode::Leaf(val) => match val {
            IrValue::CssVarRef(css) => {
                out.push_str(&format!("\"var({})\"", css));
            }
            IrValue::Raw(raw) => {
                if raw == "undefined" {
                    out.push_str("undefined");
                } else {
                    out.push_str(&format!("\"{}\"", raw));
                }
            }
        },
        IrNode::Hole => {
            out.push_str("undefined");
        }
    }
}

fn format_with_oxfmt(src: &str) -> String {
    let mut child = match Command::new("npx")
        .args(["--yes", "oxfmt", "--stdin-filepath", "theme.ts"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return src.to_string(), // silently fall back
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(src.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(out) => out,
        Err(_) => return src.to_string(),
    };

    if output.status.success() {
        String::from_utf8(output.stdout).unwrap_or_else(|_| src.to_string())
    } else {
        src.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn test_codegen() {
        let mut obj = IndexMap::new();
        obj.insert(
            "accent".to_string(),
            IrNode::Leaf(IrValue::CssVarRef("--accent".into())),
        );
        obj.insert(
            "primary".to_string(),
            IrNode::Leaf(IrValue::CssVarRef("--primary".into())),
        );
        let ir = IrNode::Object(obj);

        let ts = generate_ts(&ir);
        println!("Generated TS:\n{}", ts);
        assert!(ts.contains("export const theme"));
        assert!(ts.contains("accent: \"var(--accent)\""));
        assert!(ts.contains("primary: \"var(--primary)\""));
    }
}
