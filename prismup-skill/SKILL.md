---
name: PrismUp
description: Install and manage Prism language compilers and versions with the prismup CLI. Use when the user wants to install the Prism compiler, switch, list, or remove Prism versions, update to the latest release, work offline from cache, or troubleshoot a prismup and Prism installation.
license: BSD-3-Clause
---

# PrismUp

PrismUp is a CLI tool that installs and manages versions of
[Prism](https://github.com/sdiehl/prism), an effect-typed functional language
that compiles to native code through LLVM.

## When to use this skill

Use it when the user wants to install Prism, check the current Prism version,
list available releases, switch to or remove a specific version, upgrade to the
latest release, install without network access, or diagnose a Prism/PrismUp
setup.

## Prerequisites

- A Unix-like OS (Linux or macOS).
- `prismup` on `PATH` (`cargo install prismup`, a prebuilt release binary, or
  `just install` from sources).
- `LLVM/Clang` installed: Prism needs it to compile. On macOS, Prism < 0.17.0
  also needs the `z3` library. PrismUp does **not** install these.
- `HOME` set. PrismUp fails without it.

## Layout on disk

- `$HOME/.prismup/bin/prism` — the current Prism compiler.
- `$HOME/.prismup/bin/prism-<version>` — per-version link.
- `$HOME/.prismup/prism/<version>/prism` — installed binaries.
- `$HOME/.cache/prismup/` — cached `releases.json` and downloaded archives.

Ensure `$HOME/.prismup/bin` is on `PATH` so the `prism` command resolves.

## Commands

Run `prismup` with at most **one** action at a time.

| Action                        | Command                       |
| ----------------------------- | ----------------------------- |
| Print PrismUp version         | `prismup --version`           |
| Print current Prism version   | `prismup --current-version`   |
| Install and set latest Prism  | `prismup --upgrade`           |
| List available Prism versions | `prismup --versions-list`     |
| Install a specific version    | `prismup --install-version X` |
| Set the current version       | `prismup --set-version X`     |
| Remove a version              | `prismup --remove-version X`  |
| Refresh releases from GitHub  | `prismup --refresh`           |
| Use only the cache, no network | `prismup --offline`          |
| Clear the cache               | `prismup --cache-clear`       |
| Uninstall PrismUp             | `prismup --uninstall-prismup` |

- `X` is a `SEMVER` such as `0.22.0`.
- Running `prismup` with no arguments installs the latest Prism when none is
  installed yet.
- `--refresh` and `--offline` are mutually exclusive, as are all actions.

## Common workflows

First-time install, then add the `bin` directory to `PATH`:

```sh
prismup
export PATH="$HOME/.prismup/bin:$PATH"
```

Upgrade to the latest release:

```sh
prismup --upgrade
```

Pin a version (installs it if needed, then sets it current):

```sh
prismup --set-version 0.19.0
```

List versions, then remove one:

```sh
prismup --versions-list
prismup --remove-version 0.18.0
```

Work offline or bypass the one-hour releases cache:

```sh
prismup --install-version 0.19.0 --offline
prismup --versions-list --refresh
```

Authenticate GitHub API requests to raise the rate limit:

```sh
export GITHUB_TOKEN=...   # or GH_TOKEN
```

## Safety and rules

- Pass only one action; combining actions is rejected.
- Confirm with the user before destructive commands: `--remove-version`,
  `--cache-clear`, and `--uninstall-prismup`.
- Prefer `--offline` when there is no network or the GitHub rate limit is hit;
  a version can only be installed offline if its archive is already cached.
- Report errors verbatim: PrismUp prints failures on stderr and exits non-zero.

## Troubleshooting

- `prism: command not found` — add `$HOME/.prismup/bin` to `PATH`.
- `No Prism version set` — run `prismup` or `prismup --set-version X`.
- `no binary for <target>` — the release has no archive for this platform;
  another version must be chosen.
- Stale or partial downloads — retry with `--refresh`; clear with
  `--cache-clear`.
