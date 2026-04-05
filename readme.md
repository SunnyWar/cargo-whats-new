## cargo-whats-new

**cargo-whats-new** is a Cargo subcommand that shows you what will change when you run `cargo update` — before you actually update anything.

---

### Features
- Analyzes your project’s dependency graph
- Simulates an update in an isolated temporary workspace
- Produces a clear, structured report of:
  - Which crates would be updated
  - From which version → to which version
  - Where their repositories live
  - Where to find their changelogs
  - GitHub compare links for each version bump
- _(Future)_ Extracted changelog entries

---

This gives you a transparent, auditable view of dependency changes before you modify your real `Cargo.lock`.