## cargo-whats-new

**cargo-whats-new** is a Cargo subcommand to preview which dependencies would be updated by `cargo update`—without modifying your real project.

---

## Installation

From your local checkout:

```bash
cargo install --path .
```

---

## Usage

Run in your project directory:

```bash
cargo whats-new
```

To see more details, use:

```bash
cargo whats-new --verbose
```

To check a specific crate:

```bash
cargo whats-new <crate>
```

---

**cargo-whats-new** shows which dependencies would be updated. With `--verbose`, it also attempts to show changelist (changelog) information and links for updated crates.