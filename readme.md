## cargo-whats-new

**cargo-whats-new** is a Cargo subcommand that inspects your project's current dependency graph and can now:
- Create a temporary workspace with your `Cargo.toml`, `Cargo.lock`, and `src` directory
- Run `cargo update` in the temporary workspace
- Load and display the updated dependency graph
- Diff and report which dependencies would be updated (old → new)

This simulates what would change if you ran `cargo update` — without modifying your real project.

---

### Project Status

> **Early development:** The tool now creates a reproducible temporary workspace, copies the manifest, lock file, and source directory, runs `cargo update`, loads the updated dependency graph, and reports dependency version changes. Further reporting and changelog features are planned.

---

## Current Capabilities (Implemented)
- Loads and inspects your project’s current dependency graph using [`cargo_metadata`](https://docs.rs/cargo_metadata/)
- Creates a temporary workspace and copies `Cargo.toml`, `Cargo.lock`, and the `src` directory into it
- Runs `cargo update` in the temporary workspace (output is suppressed unless verbose)
- Loads and displays the updated dependency graph from the temp workspace
- Diffs and reports which dependencies would be updated (old → new)
- Prints repository URLs for all updated packages (verbose only)
- Prints GitHub compare links for updated crates (verbose only)
- Fetches and prints changelog entries for updated crates from GitHub (verbose only)
- Runs as a Cargo subcommand (`cargo whats-new`) once installed
- **Minimal output by default:** Only a summary of updated dependencies is shown.
- **Verbose output:** Use `-v` or `--verbose` to see detailed package lists, diffs, repository URLs, GitHub compare links, and changelog entries.
- **Single crate mode:** Run `cargo whats-new <crate>` to see only the version change and changelog diff for that crate, even without `--verbose`.

---

## Planned Features (Not Yet Implemented)
- Markdown and JSON output modes
- CI‑friendly machine‑readable reports
- Optional GitHub API integration for richer release notes

---

## Why This Tool Exists
Cargo does not provide a built‑in “dry‑run update” mode, and crates.io does not enforce changelog conventions. As a result, developers often update dependencies without knowing:
- What changed
- Whether the update is safe
- Whether it affects security posture
- Whether it introduces breaking changes

**cargo-whats-new** aims to fill that gap by giving you visibility into dependency updates _before_ they happen.

This is especially useful for:
- Security‑sensitive environments
- Teams with audit requirements
- CI pipelines
- Developers who want to understand changes before accepting them
- Anyone who prefers principled, artifact‑driven workflows

---

## Installation
From your local checkout:

```bash
cargo install --path .
```

Then run:

```bash
cargo whats-new
```

---

## Status
This project is at the “scaffolding and architecture” stage. The next major milestone is implementing richer reporting and changelog integration.

Contributions, ideas, and issue reports are welcome.