---
type: Guide
audience: user
title: Install OKF
description: Install a release, verify its checksum, and check CLI/plugin compatibility.
---
# Install OKF

**BREAKING CHANGE in CLI 0.5.0:** command input, machine output, diagnostics, and
write behavior change. Read the [0.5.0 migration guide](../reference/migrations/0.5.0.md)
and [release notes](https://github.com/abatyuk/okf/releases/tag/v0.5.0) before updating a 0.4.x installation or
automation. The OKF document specification remains v0.2. Install the 0.5.2
plugin and CLI together; the plugin accepts stable CLI versions `>=0.5.0, <0.6.0`.

Download the archive for your platform from [GitHub Releases](https://github.com/abatyuk/okf/releases):
macOS Apple Silicon (`aarch64-apple-darwin`), macOS Intel (`x86_64-apple-darwin`),
Linux ARM64 (`aarch64-unknown-linux-musl`), or Linux Intel/AMD (`x86_64-unknown-linux-musl`).
Download `SHA256SUMS` from the same release and compare the archive's SHA-256 using
`shasum -a 256 ARCHIVE` (macOS) or `sha256sum ARCHIVE` (Linux). Extract the archive,
then copy `okf` into a directory on PATH, for example:

```sh
mkdir -p "$HOME/.local/bin"
cp okf "$HOME/.local/bin/okf"
chmod +x "$HOME/.local/bin/okf"
export PATH="$HOME/.local/bin:$PATH"
okf version
```

Alternatively, download the standalone installer from a **published release**, inspect it,
then run it with an explicit version. This example installs or updates to 0.5.2:

```sh
curl --fail --location --proto '=https' --proto-redir '=https' \
  https://github.com/abatyuk/okf/releases/download/v0.5.2/install.sh -o install.sh
# Review install.sh before running it.
sh install.sh 0.5.2
```

The optional installer detects your platform, verifies the archive checksum, checks that the
binary runs, and installs to `~/.local/bin`. It needs curl, tar, and sha256sum or shasum;
it does not need Rust, Python, GitHub CLI, or an existing OKF installation. Supply a second
argument to choose another installation directory. To update or roll back, select that
release's version. It does not edit shell configuration or use administrator privileges.
Add `~/.local/bin` to PATH in your shell configuration if needed. An older `okf` elsewhere
on PATH can take precedence; check `command -v okf` and `okf version`.

Skills **only check compatibility** and show these instructions when necessary. They never
run the installer or update the CLI. The current plugin supports stable CLI versions
`>=0.5.0, <0.6.0`; the OKF document specification remains v0.2.

Starting with 0.4.1, macOS release binaries are Developer ID signed and notarized by Apple.
The standalone executable has no stapled ticket, so Gatekeeper may need internet access
to check notarization. macOS builds use deployment target 13.0; see release notes for
tested platforms and limitations.
Git-based operations require `git` on PATH.

Official binaries include `url-sources` for explicit remote artifact retrieval with
`artifact show --fetch`. URL fingerprint refresh is not yet wired to the CLI network client.
Local operations work offline. See [release maintenance](../contributing/release-process.md) for the publishing process.
