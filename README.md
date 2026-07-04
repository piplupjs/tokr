# Tokr

[![CI](https://github.com/sadik-malik/tokr/actions/workflows/ci.yml/badge.svg)](https://github.com/sadik-malik/tokr/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

---

## About

**Tokr** is a high‑performance, Rust‑based command‑line compiler that transforms token definition files (e.g. CSS & SCSS variables) into **TypeScript** or **JavaScript** modules. It is purpose‑built for front‑end teams that need a single source of truth for design tokens, enabling type‑safe imports in modern JavaScript/TypeScript projects.

The tool provides:

- **Fast, parallel compilation** driven by `rayon`.
- **Configurable linting** (kebab‑case variable names, hexadecimal case, …).
- **Strict / watch modes** for CI pipelines and local development.
- **Automatic output mapping** that mirrors the input directory structure.

The repository is a **workspace** containing a collection of crates that implement the lexer, parser, semantic analysis, intermediate representation, code generation, diagnostics, and driver orchestration.

---

## Features

| Feature | Description |
|---------|-------------|
| **Zero‑install binaries** | Pre‑built tarballs for macOS, Linux and Windows are published on each release.
| **Configuration file** (`tokr.json`) | Declare input glob patterns, output file name, lint rules, and compiler options in a single JSON document.
| **Watch mode** | Continuously re‑compile on source changes (useful during design‑token authoring).
| **Strict mode** | Treat warnings as errors to enforce a clean token set.
| **Check mode** | Verify that generated files are up‑to‑date without overwriting them.
| **Extensible linting** | Enable/disable specific lint rules via `tokr.json`.
| **Multiple output formats** | Generate **TypeScript** (`.ts`) or **JavaScript** (`.js`) along with optional declaration files (`.d.ts`).

---

## Getting Started

### Prerequisites

- **Rust** (stable) – required only if you want to build from source.
- `curl` and `tar` – for downloading the pre‑built binary.

### Installing a pre‑built binary (recommended)

```bash
# Download the latest release for the current platform
curl -LO "https://github.com/sadik-malik/tokr/releases/latest/download/tokr-$(uname -s)-$(uname -m).tar.gz"

# Extract the archive and install globally
sudo tar -xzf tokr-*.tar.gz -C /usr/local/bin
```

Verify the installation:

```bash
tokr --version
```

### Building from source

```bash
git clone https://github.com/sadik-malik/tokr.git
cd tokr
cargo build --release
sudo cp target/release/tokr /usr/local/bin/
```

---

## Usage

```bash
# Show the full help text
$tokr --help
```

### Common commands

| Command | Description |
|---------|-------------|
| `tokr` | Run the compiler with options specified in `tokr.json`.
| `tokr --watch` | Watch the source directory and re‑compile on changes.
| `tokr --check` | Verify that generated files are up‑to‑date (no writes).
| `tokr --strict` | Promote warnings to errors.
| `tokr --out-dir <dir>` | Override the output directory defined in the config.
| `tokr --config <path>` | Use a custom configuration file (default: `tokr.json`).
| `tokr --input <glob>` | Provide input glob patterns via the CLI (overrides config).
```

All flags are also available in the configuration file (see next section).

---

## Configuration

Tokr reads a JSON configuration file (`tokr.json` by default). An example configuration is shown below:

```json
{
  "input": ["src/**/*.scss"],
  "output": "dist/theme.ts",
  "options": {
    "strict": true,
    "format": "ts",
    "declaration": true,
    "order": {
      "": ["colors", "typography"]
    },
    "lint": {
      "kebab-case-vars": true,
      "hex-case": "lower"
    }
  }
}
```

### Configuration fields

| Field | Type | Description |
|-------|------|-------------|
| `$schema` | `string` (optional) | URL to a JSON‑Schema that validates the file.
| `input` | `array<string>` | Glob patterns for token source files.
| `output` | `string` (optional) | Path of the primary generated file.
| `options.strict` | `bool` (default `false`) | Fail the compilation on warnings.
| `options.format` | `"ts"` or `"js"` (default `"ts"`) | Choose the output language.
| `options.declaration` | `bool` (default `false`) | Emit a `.d.ts` declaration file (only for TS output).
| `options.order` | `object` (optional) | Mapping of token categories to a desired ordering.
| `options.lint` | `object` | Lint rule configuration (`kebab-case-vars`, `hex-case`).

The configuration file can be placed at the repository root or supplied via `--config <path>`.

---

## Development

The workspace consists of the following crates (each lives under `crates/`):

- `tokr-config` – JSON deserialization of the configuration file.
- `tokr-lexer` – Tokenizer for the source language.
- `tokr-parser` – Syntax parsing.
- `tokr-sema` – Semantic analysis and symbol table creation.
- `tokr-ir` – Intermediate representation and compilation passes.
- `tokr-codegen` – Generation of TypeScript/JavaScript code.
- `tokr-driver` – Orchestrates discovery, compilation, linting, and file output.
- `tokr-diagnostics` – Unified diagnostics rendering.
- `tokr-linter` / `tokr-linter-rules` – Linting infrastructure.
- `tokr-ast` – Abstract syntax‑tree definitions.

### Running the test suite

```bash
cargo test --all
```

### Adding a new lint rule

1. Implement the rule in `crates/tokr-linter-rules` implementing `tokr_linter::LintRule`.
2. Register it in `tokr-driver/src/lib.rs` where the rule vector is constructed.
3. Add configuration support in `tokr-config/src/lib.rs` if needed.

---

## Contributing

We love contributions! Please read our [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on:

- Setting up a development environment.
- Running the CI locally.
- Submitting pull requests.
- Coding style and commit conventions.

---

## License

Tokr is licensed under the **Apache License 2.0**. See the full license text in the [LICENSE](LICENSE) file.
