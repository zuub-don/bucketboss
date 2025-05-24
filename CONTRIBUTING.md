# Contributing to BucketBoss

Thank you for considering contributing to BucketBoss! We welcome all contributions, whether they're bug reports, feature requests, documentation improvements, or code contributions.

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

- Check if the bug has already been reported in the [issue tracker](https://github.com/copyleftdev/bucketboss/issues).
- If you're unable to find an open issue addressing the problem, [open a new one](https://github.com/copyleftdev/bucketboss/issues/new).
- Be sure to include a title and clear description, with as much relevant information as possible.
- If possible, provide a minimal code sample that reproduces the issue.

### Suggesting Enhancements

- Use the issue tracker to suggest new features or improvements.
- Clearly describe the feature/improvement and why you believe it would be useful.
- If possible, provide examples of how the feature would be used.

### Pull Requests

1. Fork the repository and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. If you've changed APIs, update the documentation.
4. Ensure the test suite passes: `cargo test --all-features`
5. Format your code: `cargo fmt`
6. Run clippy: `cargo clippy --all-targets --all-features -- -D warnings`
7. Make sure your code is properly documented.
8. Update the CHANGELOG.md if your changes are user-facing.
9. Create a pull request with a clear description of your changes.

## Development Setup

1. Fork the repository.
2. Clone your fork: `git clone https://github.com/your-username/bucketboss.git`
3. Navigate to the project directory: `cd bucketboss`
4. Build the project: `cargo build`
5. Run tests: `cargo test`

## Code Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `rustfmt` to format your code.
- Run `cargo clippy` to catch common mistakes and improve your code.
- Document all public APIs using Rustdoc comments.

## License

By contributing to BucketBoss, you agree that your contributions will be licensed under both the MIT and Apache 2.0 licenses, as specified in the project's root directory.
