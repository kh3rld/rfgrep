rfgrep v0.2.1 — 2025-08-24

Summary
-------
This patch prepares and stabilizes the v0.2.1 release. It removes the `anyhow` dependency, unifies crate error handling, returns structured search results for correct JSON output, tightens CI, and fixes clippy/style issues.

Highlights
---------
- Errors & types
  - Removed `anyhow` and the `Anyhow` variant from the crate error type.
  - Adopted `crate::error::Result<T>` and `RfgrepError` as the canonical result/error types.

- Search & output
  - `search_file` now returns structured `SearchMatch` objects including `path`, `line_number`, `matched_text`, `context_before`, and `context_after`.
  - JSON output now serializes structured matches for accurate, machine-friendly results.

- CI / Dev tooling
  - Rewrote `.github/workflows/ci.yml` to run tests across a matrix (Linux/macOS/Windows), run formatting/clippy checks, build artifacts, and optionally run the shell harness under Xvfb.
  - CI enforces `RUSTFLAGS='-D warnings'` to catch lints early.

- Code quality
  - Ran `cargo fmt` and fixed clippy warnings across the repo.
  - Updated scripts to avoid reintroducing `anyhow`.

- Docs
  - Tidied `man/rfgrep.1` to prefer `--path` and clarified positional compatibility.

Notes for users and integrators
------------------------------
- CI is strict about lints. Before pushing changes, run locally:

  ```bash
  cargo fmt --all
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all-features --workspace
  ```

- If you publish from CI, ensure the `CARGO_REGISTRY_TOKEN` secret is set in repository settings.

How to create the release (two options)
---------------------------------------
Option A — using GitHub CLI (recommended):

1. Tag locally and push the tag:

```bash
git tag -a v0.2.1 -m "rfgrep v0.2.1"
git push origin v0.2.1
```

2. Create the release using the `gh` CLI (this will open a release on GitHub):

```bash
gh release create v0.2.1 --notes-file GITHUB_RELEASE_v0.2.1.md
```

Option B — using the GitHub API with `GITHUB_TOKEN` (CI-compatible):

```bash
# set GITHUB_TOKEN in environment (or use repo secret)
API_JSON=$(jq -n --arg tag "v0.2.1" --arg name "rfgrep v0.2.1" --arg body "$(sed 's/"/\\"/g' GITHUB_RELEASE_v0.2.1.md)" '{tag_name: $tag, name: $name, body: $body, draft: false, prerelease: false}')

curl -H "Authorization: token $GITHUB_TOKEN" \
     -H "Content-Type: application/json" \
     -d "$API_JSON" \
     "https://api.github.com/repos/kh3rld/rfgrep/releases"
```

(Adjust the owner/repo in the URL if different.)

Files changed in this release
-----------------------------
- `src/error.rs` — removed `Anyhow` variant and standardized crate result type.
- `src/processor.rs` / `src/search.rs` — refactored search to produce structured `SearchMatch` results.
- `src/main.rs` / `src/output_formats.rs` — wired the `SearchMatch` into JSON/text output.
- `.github/workflows/ci.yml` — CI workflow rewritten.
- `man/rfgrep.1` — synopsis updated.
- `scripts/run_benchmarks.rs` — script updated to avoid `anyhow`.

Acknowledgements
----------------
Thanks to contributors for the refactor and CI improvements. Please report regressions via issues or PRs.
