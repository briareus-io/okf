---
type: Reference
audience: contributor
title: Develop and document OKF
description: Build the CLI, regenerate its references, and validate documentation changes.
sources:
- resource: xtask/src/main.rs
  kind: git-path
---
# Develop and document OKF

**BREAKING CHANGE:** `cargo xtask install` builds and installs the 0.5.2 CLI.
Review the [migration guide](../reference/migrations/0.5.0.md) before running it over
an existing installation.

```sh
cargo xtask install          # build + install `okf` to ~/.cargo/bin, then regenerate docs
```

`cargo xtask install` wraps `cargo install --path crates/okf-cli --locked` and then regenerates the
[generated references](#generated-documentation) so they match the freshly built CLI. Prefer it
over a bare `cargo install`, which installs the binary but leaves the references untouched.
The locked dependency set matches the tested workspace build.

Build without installing:

```sh
cargo xtask build --release  # cargo build --release, then regenerate docs (binary: target/release/okf)
cargo build --release        # plain build, no doc regeneration
# optional remote artifact retrieval (`artifact show --fetch`):
cargo build --release -p okf-cli --features url-sources
```

On macOS outside CI, `cargo xtask build` and `cargo xtask install` ad-hoc sign local
Mach-O executables with `codesign --force --sign -` and verify the resulting signatures.
Build signing follows Cargo's reported artifact paths, including custom profiles and target
directories. Install signing honors `CARGO_INSTALL_ROOT`, Cargo's `install.root` configuration,
and `CARGO_HOME`. Signing is skipped whenever `CI` is set, on other operating systems,
and for cross-compiled non-Mach-O executables. Release signing and notarization stay in
the release workflow. Plain `cargo build` and `cargo install` do not run this signing step.

Requires a recent stable Rust. `git` on `PATH` is needed only for git-based source kinds and
`okf diff`; everything else works without it.



## Generated documentation

`okf schema` supplies command arguments, supported globals, write capability, and output contracts.
`cargo xtask docs` updates the shared [CLI contracts](../reference/cli-contracts.md),
the Arguments section of every [command](../commands/index.md), and the focused CLI reference
packaged with each skill. Edit `xtask/src/main.rs` for shared usage notes; keep task guidance in this bundle.
`xtask/skill-scenarios.json` selects each skill's reference subset.

```sh
cargo xtask docs
cargo xtask docs --check
cargo xtask skills
cargo test --workspace --locked
python3 scripts/check-docs.py
```

The documentation check validates the bundle, local navigation, declared relationships and
worked artifacts in disposable bundles. Run it from the repository root after building the CLI.
Use `OKF_DOCS_BIN=target/debug/okf` to choose the binary explicitly.

## Placement and maintenance

The root index serves readers and contributors. Put task instructions in guides, reusable
explanations in concepts, exact behavior in reference/commands, and implementation reasoning
under contributing. Keep Rust API types in `cargo doc --workspace --no-deps`, not mirrored concepts.
Plugin `SKILL.md` files remain the executable agent instructions; bundle skill pages help users choose them.

Indexes are curated. Do not run `okf docs knowledge --format index` over them: it replaces
navigation prose. Add a local index link whenever adding a page, and review source-backed
pages when implementation changes. A fingerprint refresh records a new baseline, not verification.

Keep source references pointed at current repository files. This bundle does not store source
fingerprints or Git snapshot links; its local lint policy disables only `source_unrecorded`.
Use source-impact queries and direct review for maintenance. Do not append verification events
merely because checks pass.
