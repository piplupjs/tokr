use clap::Parser;
use std::path::{Path, PathBuf};
use tokr_config::TokrConfig;
use tokr_driver::discovery::FileDiscovery;
use tokr_ir::PassConfig;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,

    #[arg(short = 'i', long)]
    input: Option<Vec<String>>,

    #[arg(long = "out-dir")]
    out_dir: Option<PathBuf>,

    #[arg(long)]
    strict: bool,

    #[arg(long)]
    check: bool,

    #[arg(short = 'w', long)]
    watch: bool,
}

fn main() {
    let cli = Cli::parse();

    let config_path = cli.config.unwrap_or_else(|| PathBuf::from("tokr.json"));

    let config = if config_path.exists() {
        let json = std::fs::read_to_string(&config_path).expect("failed to read config");
        TokrConfig::load(&json).expect("failed to parse config")
    } else {
        TokrConfig {
            schema: None,
            input: Vec::new(),
            output: None,
            options: Default::default(),
        }
    };

    let inputs = if let Some(cli_inputs) = cli.input {
        cli_inputs
    } else {
        config.input
    };

    if inputs.is_empty() {
        eprintln!("No input files specified. Provide them in tokr.json or via --input");
        std::process::exit(1);
    }

    let out_dir = cli
        .out_dir
        .or_else(|| config.output.map(PathBuf::from))
        .unwrap_or_else(PathBuf::new);

    let discovery = FileDiscovery::new(&inputs).expect("invalid glob pattern");

    let mut order = std::collections::HashMap::new();
    if let Some(cfg_order) = config.options.order {
        order = cfg_order;
    }

    let pass_cfg = PassConfig {
        strict: cli.strict || config.options.strict,
        order_table: order,
    };

    // Treat the directory of `tokr.json` as the root, or cwd if using the default
    let root = config_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_path_buf();
    let root = if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root
    };

    let is_js = config.options.format == tokr_config::OutputFormat::Js;
    let declaration = config.options.declaration;
    let lint_cfg = Some(&config.options.lint);

    let compile_once = || {
        let result = tokr_driver::compile_project(
            &root,
            &out_dir,
            &discovery,
            &pass_cfg,
            config.options.allow_name_mismatch,
            lint_cfg,
            is_js,
            declaration,
            cli.check,
        );

        if !result.errors.is_empty() {
            let mut has_fatal = false;
            for (path, diags) in result.errors {
                let src = std::fs::read_to_string(&path).unwrap_or_default();
                let all = diags.into_vec();
                tokr_diagnostics::render_diagnostics(&path.to_string_lossy(), &src, &all);
                if all
                    .iter()
                    .any(|d| d.severity == tokr_diagnostics::Severity::Error)
                {
                    has_fatal = true;
                }
            }
            if has_fatal {
                eprintln!("Compilation failed due to errors.");
                if !cli.watch {
                    std::process::exit(1);
                }
            }
        } else {
            println!(
                "Successfully compiled {} files to {}",
                result.success_count,
                out_dir.display()
            );
        }
    };

    compile_once();

    if cli.watch {
        use notify::{EventKind, RecursiveMode, Watcher};
        use std::sync::mpsc::channel;

        println!("Watching for changes...");
        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(tx).expect("failed to create watcher");
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .expect("failed to watch directory");

        for res in rx {
            match res {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    ) {
                        println!("Changes detected, recompiling...");
                        compile_once();
                        println!("Watching for changes...");
                    }
                }
                Err(e) => eprintln!("watch error: {:?}", e),
            }
        }
    }
}
