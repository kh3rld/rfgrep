Release v0.2.1 — 2025-08-24
=================================

Summary
-------
This patch prepares the project for v0.2.1. It focuses on removing the `anyhow` dependency and consolidating error handling, returning structured search results for accurate JSON output, CI improvements, and general code hygiene (formatting and clippy fixes).

Highlights
---------
- Errors & types
  - Removed `anyhow` and the `Anyhow` variant from the crate error type.
  - Adopted `crate::error::Result<T>` and `RfgrepError` as the canonical result/error types.

- Search & output
  - `search_file` now returns structured `SearchMatch` objects including `path`, `line`, `context_before`, `context_after`, and match ranges.
  - JSON output now serializes structured matches for accurate, machine-friendly results.

- CI / Dev tooling
  - Rewrote `.github/workflows/ci.yml` to run tests (matrix), linting, builds, optional integration harness (Xvfb), and gated publish.
  - CI enforces `RUSTFLAGS='-D warnings'` to catch lints early.

- Code quality
  - Ran `cargo fmt` and fixed clippy warnings across the repo (style fixes in `main.rs`, `config.rs`, and `processor.rs`).
  - Updated scripts (e.g., `scripts/run_benchmarks.rs`) to avoid reintroducing `anyhow`.

- Docs
  - Tidied `man/rfgrep.1` to prefer `--path` in the SYNOPSIS and clarified positional compatibility.

Cleanup recommendations performed / suggested
-----------------------------------------
- Remove generated build artifacts from the repository (local `target/`, top-level `rfgrep` binary, `robustness_test_*.txt` logs). Add them to `.gitignore` if not already ignored.
- Consider moving large bench data (`bench_data/binary.bin`) to a release asset or a separate storage location (or use Git LFS) if it's required for reproducing benchmarks.

Upgrade notes
-------------
- CI uses strict warnings-as-errors; run the following locally before committing:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --workspace
```

- If you publish to crates.io from CI, ensure `CARGO_REGISTRY_TOKEN` is configured in repository Secrets.

Suggested release commands
--------------------------
Run locally to tag and push the release (replace `main` with your release branch if needed):

```bash
git add Cargo.toml CHANGELOG.md Release0.2.1.md
git commit -m "chore(release): v0.2.1"
git tag -a v0.2.1 -m "rfgrep v0.2.1"
git push origin --follow-tags
```

Files/areas to review after release
----------------------------------
- `scripts/` — ensure helper scripts still behave without `anyhow`.
- `man/` — review generated man pages if `make -C man gzip` runs in CI.
- `bench_data/` — confirm benchmarks remain reproducible or provide instructions for obtaining large sample artifacts.

Contact
-------
If you'd like, I can (pick one):
- Bump `Cargo.toml` to v0.2.1 and create the git tag + push it.
- Clean up recommended files (remove tracked `target/`, binary, logs) and add `.gitignore` entries, then commit.
- Draft the GitHub Release body from this note and publish it.


Small changelog excerpt
----------------------
- Removed `anyhow` and canonicalized crate error handling.
- Return structured `SearchMatch` objects from search routines and fixed JSON output.
- Rewrote CI workflow and added optional Xvfb integration harness.
- Various clippy/style fixes and minor documentation updates.


