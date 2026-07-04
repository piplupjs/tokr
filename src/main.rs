//! Root binary that simply forwards to the CLI crate.

fn main() {
    // The CLI crate `cli/tokr` provides a binary with its own `main`.
    // We can invoke it by re-exporting its `main` function via a library.
    // For simplicity, we just call the binary as a subprocess.
    // This keeps the root package lightweight and ensures `cargo install tokr`
    // builds the same binary as the one in `cli/tokr`.

    // If the CLI crate also exposes a library entry point, you could `cli_tokr::main()`.
    // Here we just run the binary directly.
    let status = std::process::Command::new("cargo")
        .args(["run", "-p", "cli-tokr", "--"])
        .status()
        .expect("failed to execute cli-tokr");
    std::process::exit(status.code().unwrap_or(1));
}
