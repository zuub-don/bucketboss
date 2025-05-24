# Release Checklist

## Pre-Release

- [ ] Ensure all tests pass: `cargo test --all-features`
- [ ] Run benchmarks: `cargo bench`
- [ ] Update `CHANGELOG.md` with all changes since last release
- [ ] Update version in `Cargo.toml` following semantic versioning
- [ ] Update documentation and README if needed
- [ ] Ensure all new features are documented
- [ ] Run `cargo doc --no-deps --open` to verify documentation builds correctly
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo fmt --all -- --check`
- [ ] Test with `--no-default-features`
- [ ] Test on different platforms (Linux, Windows, macOS)

## Release Process

- [ ] Create a signed tag: `git tag -s vX.Y.Z -m "vX.Y.Z"`
- [ ] Push the tag: `git push origin vX.Y.Z`
- [ ] Publish to crates.io: `cargo publish --no-verify`

## Post-Release

- [ ] Update version in `Cargo.toml` to the next development version
- [ ] Create a new unreleased section in `CHANGELOG.md`
- [ ] Push changes: `git push origin main`
- [ ] Create a GitHub release with release notes
- [ ] Announce the release (blog post, social media, etc.)

## Verification

- [ ] Verify the package on crates.io
- [ ] Verify documentation on docs.rs
- [ ] Test the published package in a new project
- [ ] Check that all CI/CD pipelines passed
