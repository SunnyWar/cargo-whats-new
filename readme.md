## cargo-whats-new

**cargo-whats-new** is a Cargo subcommand that inspects your project's current dependency graph and can now create a temporary workspace with your `Cargo.toml` and `Cargo.lock` files. This is the first step toward simulating what would change if you ran `cargo update` — without modifying your real project.

---

### Project Status

> **Early development:** The tool now creates a reproducible temporary workspace and copies the manifest and lock files. Update simulation and diffing are not yet available, but the foundation is in place.

---

## Current Capabilities (Implemented)
- Loads and inspects your project’s current dependency graph using [`cargo_metadata`](https://docs.rs/cargo_metadata/)
- Prints the workspace root and lists all packages (name, version, manifest path)
- Creates a temporary workspace and copies `Cargo.toml` and `Cargo.lock` into it
- Runs as a Cargo subcommand (`cargo whats-new`) once installed

**Note:** The tool does _not_ yet simulate updates or produce diffs. Those features are under active development.

---

## Planned Features (Not Yet Implemented)
- Run `cargo update` inside the temp workspace
- Load the updated dependency graph
- Diff the before/after versions
- Show which crates would be updated
- Show version changes (old → new)
- Identify each crate’s repository
- Locate changelogs automatically
- Generate GitHub compare links for each version bump
- Extract changelog entries automatically
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
This project is at the “scaffolding and architecture” stage. The next major milestone is implementing the temporary‑workspace update simulation and version diffing.

Contributions, ideas, and issue reports are welcome.