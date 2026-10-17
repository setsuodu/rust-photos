# rust-photos

Rust photos utility.

## Build

```bash
cargo build --release
```

## Release

Push a tag to trigger CI, which builds binaries and uploads them to GitHub Releases:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Or create a Release on GitHub; the workflow runs on `v*` tags.
