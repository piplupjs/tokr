# Tokr

[![CI](https://github.com/sadik-malik/tokr/actions/workflows/ci.yml/badge.svg)](https://github.com/sadik-malik/tokr/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

A Rust workspace for the **Tokr** project that builds a command‑line binary. The repository hosts the source and a CI pipeline that publishes pre‑compiled binaries for macOS, Linux, and Windows.

## Download Binaries

```bash
# Download the latest release for your platform
curl -LO "https://github.com/sadik-malik/tokr/releases/latest/download/tokr-$(uname -s)-$(uname -m).tar.gz"
# Extract and run
tar -xzf tokr-*.tar.gz
./tokr --help
```

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache-2.0 License – see the [LICENSE](LICENSE) file.
