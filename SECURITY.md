# Security policy and advisory notes

This repository documents how we handle security advisories and tracked exceptions for third-party crates.

## RUSTSEC-2024-0436 — `paste` (no longer maintained)

- Advisory ID: RUSTSEC-2024-0436
- Crate: `paste` (proc-macro)
- Why it appears: `paste` is a transitive dependency pulled in by `ratatui` (a terminal UI crate used by `rfgrep` for TUI features).

Why we have an exception
- `paste` is not a direct dependency of `rfgrep`.
- We currently use `ratatui` for optional TUI features; `paste` is pulled in as a proc-macro build-time dependency by that crate.
- We assessed the risk as acceptable for the following reasons:
  - `paste` is used at build time as a proc-macro to produce code for `ratatui` and does not run in production runtimes (no network or persistence access in our shipped binary beyond normal codegen).
  - The crate is transitive and we do not directly invoke `paste` in our code.
  - We have pinned `ratatui` in `Cargo.lock` and monitor upstream changes.

Where it's recorded
- We have an allow rule in `.cargo/config.toml` to ignore the advisory during automated checks:

  [advisories]
  ignore = ["RUSTSEC-2024-0436", "RUSTSEC-2024-0375", "RUSTSEC-2021-0145"]

- The advisory was also noted in our `CHANGELOG.md`

What we did now
- Documented the advisory and rationale here.
- Preserved the `.cargo/config.toml` allow rule (existing in repo) so automated tooling does not fail CI for this advisory.

How to review further
- Run `cargo audit` in your environment to see the advisory and verify the allow rule behaves as expected.
- To remove the exception proactively, try updating `ratatui` in `Cargo.toml` to a newer minor/.x release and run `cargo update -p ratatui` and `cargo tree -i paste` to verify the transitive dependency was removed.
