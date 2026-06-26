# AGENTS.md

## Dependency provenance

Keep every third-party input used by this repository reproducible and tied to
immutable provenance.

- Pin GitHub Actions to full 40-character commit SHAs. Add a trailing comment
  with the corresponding release tag so humans can identify the version.
- Pin container images by `sha256` digest rather than mutable tags. An explicit
  tag may remain in the image name for readability.
- Verify downloaded archives and binaries with a committed SHA-256 checksum
  before extracting or executing them. Downloads used by CI should live outside
  the source tree, such as under `$RUNNER_TEMP`.
- Keep Python CI and development tools in `requirements-ci.in`, and commit the
  fully resolved `requirements-ci.txt` with hashes for every direct and
  transitive package. Install it with `pip --require-hashes`.
- Commit `Cargo.lock`, retain registry checksums, and use `cargo --locked` in CI
  and publishing workflows. Normal version requirements remain in `Cargo.toml`
  for downstream library compatibility.
- Pin language toolchain versions used by CI. Operating-system packages may be
  installed through the distribution package manager, whose signed repository
  metadata provides their provenance.

When updating a dependency, update its human-readable version annotation and
immutable SHA or digest together. Verify the referenced artifact before
committing the new pin.
