# Contributing Guidelines

Thank you for considering contributing to **Tokr**!

## How to Contribute
1. **Fork the repository** and clone your fork.
2. Create a **feature branch**:
   ```bash
   git checkout -b feature/your-feature
   ```
3. Make your changes, ensuring they follow the project's style:
   - Run `cargo fmt -- --check`.
   - Run `cargo clippy -- -D warnings`.
   - Add or update tests.
4. Run the full test suite:
   ```bash
   cargo test --locked
   ```
5. Commit with a **conventional commit** message (e.g., `feat: add new parser`).
6. Push and open a **Pull Request** targeting the `main` branch.

## Code Style
- Use `rustfmt` formatting.
- Keep lint warnings at zero (`cargo clippy`).
- Document public APIs with doc comments.

## Testing
- Write unit tests in the `tests/` directory or inline `#[cfg(test)]` modules.
- Ensure `cargo test` passes on CI.

## Release Process
- See the [RELEASE.md](RELEASE.md) for the steps to create a new version.

---
*We appreciate all contributions, big or small!*
