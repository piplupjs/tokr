//! `tokr-driver` is the orchestration layer of the Tokr token compiler.
//!
//! It is responsible for discovering CSS & SASS files, mapping their output paths,
//! coordinating the lexing -> parsing -> sema -> ir -> codegen pipeline,
//! and writing the TypeScript output files back to disk.
//! It supports parallel compilation via `rayon` for massive workspaces.

pub mod discovery;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tokr_diagnostics::DiagnosticBag;
use tokr_ir::PassConfig;

pub fn compile(
    src: &str,
    cfg: &PassConfig,
    lint_cfg: Option<&tokr_config::LintConfig>,
    is_js: bool,
    declaration: bool,
) -> Result<(String, Option<String>), DiagnosticBag> {
    let (tokens, lex_diags) = tokr_lexer::Lexer::new(src).tokenize();
    let mut diags = DiagnosticBag::default();
    diags.extend(lex_diags);

    let parser = tokr_parser::Parser::new(src, &tokens, &mut diags);
    let file = parser.parse_file();

    if diags.has_errors() {
        return Err(diags);
    }

    if let Some(l_cfg) = lint_cfg {
        let mut rules: Vec<Box<dyn tokr_linter::LintRule>> = Vec::new();

        if l_cfg.kebab_case_vars.unwrap_or(true) {
            rules.push(Box::new(tokr_linter_rules::KebabCaseVarsRule));
        }

        if let Some(case) = &l_cfg.hex_case {
            rules.push(Box::new(tokr_linter_rules::HexCaseRule::new(case.clone())));
        }

        let linter = tokr_linter::Linter::new(rules);
        linter.lint_file(&file, &mut diags);
    }

    let sym_table = tokr_sema::analyze(&file, &mut diags);

    if cfg.strict {
        diags.promote_warnings_to_errors();
    }

    if diags.has_errors() {
        return Err(diags);
    }

    if diags.has_errors() {
        return Err(diags);
    }

    let ir = tokr_ir::lower(&sym_table);
    let ir = tokr_ir::run_passes(ir, cfg);

    let main_code = if is_js {
        tokr_codegen::generate_js(&ir)
    } else {
        tokr_codegen::generate_ts(&ir)
    };

    let decl_code = if declaration && is_js {
        Some(tokr_codegen::generate_dts(&ir))
    } else {
        None
    };

    Ok((main_code, decl_code))
}

pub struct ProjectResult {
    pub success_count: usize,
    pub errors: Vec<(PathBuf, DiagnosticBag)>,
}

pub fn compile_project(
    root: &Path,
    output_dir: &Path,
    discovery: &discovery::FileDiscovery,
    cfg: &PassConfig,
    lint_cfg: Option<&tokr_config::LintConfig>,
    is_js: bool,
    declaration: bool,
    check_mode: bool,
) -> ProjectResult {
    let files = discovery.discover(root);

    let results: Vec<_> = files
        .into_par_iter()
        .map(|path| {
            let src = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    let mut bag = DiagnosticBag::default();
                    bag.error("IO_ERROR", e.to_string(), tokr_span::Span::new(0, 0));
                    return Err((path, bag));
                }
            };

            match compile(&src, cfg, lint_cfg, is_js, declaration) {
                Ok((main_code, decl_code)) => {
                    let main_ext = if is_js { "js" } else { "ts" };

                    if let Some(out_path) =
                        discovery::map_output_path(&path, root, output_dir, main_ext)
                    {
                        if check_mode {
                            match fs::read_to_string(&out_path) {
                                Ok(existing) if existing == main_code => {} // matches, ok
                                Ok(_existing) => {
                                    let mut bag = DiagnosticBag::default();
                                    bag.error(
                                        "DRIFT_ERROR",
                                        format!("File {} is out of date", out_path.display()),
                                        tokr_span::Span::new(0, 0),
                                    );
                                    return Err((path, bag));
                                }
                                Err(_) => {
                                    let mut bag = DiagnosticBag::default();
                                    bag.error(
                                        "DRIFT_ERROR",
                                        format!("File {} is missing", out_path.display()),
                                        tokr_span::Span::new(0, 0),
                                    );
                                    return Err((path, bag));
                                }
                            }
                        } else {
                            if let Some(parent) = out_path.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                            if let Err(e) = fs::write(&out_path, main_code) {
                                let mut bag = DiagnosticBag::default();
                                bag.error("IO_ERROR", e.to_string(), tokr_span::Span::new(0, 0));
                                return Err((path, bag));
                            }
                        }
                    }

                    if let Some(dts_code) = decl_code {
                        if let Some(dts_path) =
                            discovery::map_output_path(&path, root, output_dir, "d.ts")
                        {
                            if check_mode {
                                match fs::read_to_string(&dts_path) {
                                    Ok(existing) if existing == dts_code => {}
                                    Ok(_existing) => {
                                        let mut bag = DiagnosticBag::default();
                                        bag.error(
                                            "DRIFT_ERROR",
                                            format!("File {} is out of date", dts_path.display()),
                                            tokr_span::Span::new(0, 0),
                                        );
                                        return Err((path, bag));
                                    }
                                    Err(_) => {
                                        let mut bag = DiagnosticBag::default();
                                        bag.error(
                                            "DRIFT_ERROR",
                                            format!("File {} is missing", dts_path.display()),
                                            tokr_span::Span::new(0, 0),
                                        );
                                        return Err((path, bag));
                                    }
                                }
                            } else {
                                if let Some(parent) = dts_path.parent() {
                                    let _ = fs::create_dir_all(parent);
                                }
                                if let Err(e) = fs::write(&dts_path, dts_code) {
                                    let mut bag = DiagnosticBag::default();
                                    bag.error(
                                        "IO_ERROR",
                                        e.to_string(),
                                        tokr_span::Span::new(0, 0),
                                    );
                                    return Err((path, bag));
                                }
                            }
                        }
                    }
                    Ok(())
                }
                Err(diags) => Err((path, diags)),
            }
        })
        .collect();

    let mut success_count = 0;
    let mut errors = Vec::new();

    for res in results {
        match res {
            Ok(_) => success_count += 1,
            Err(e) => errors.push(e),
        }
    }

    ProjectResult {
        success_count,
        errors,
    }
}
