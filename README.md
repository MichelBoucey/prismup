# PrismUp [![CI](https://github.com/MichelBoucey/prismup/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/MichelBoucey/prismup/actions/workflows/ci.yml)

`PrismUp` is a CLI tool to install and manage versions of [Prism](https://github.com/sdiehl/prism), "an effect-typed functional language that compiles to native code through LLVM".

## 1. OS Platforms

`PrismUp` targets Unix-like OS.

## 2. Dependencies

`Prism`, as compiler, needs `LLVM/Clang` to run on your system (on `macOS`, `Prism` < 0.17.0 needs also `z3` library to be installed).  `PrismUp` don't install those dependencies but only `Prism`. See [the README of the Prism's project](https://github.com/sdiehl/prism/blob/main/README.md).

## 3. Installation of PrismUp

### 3.1. From Releases

Just download from [releases](https://github.com/MichelBoucey/prismup/releases).

### 3.2. From crates.io

```
cargo install prismup
```

### 3.3. From sources

```
just install
```

Install `prismup` to `~/.cargo/bin/`. Add this `bin/` directory to your `PATH` if it's not already done.

## 4. Initial installation of Prism

At its first run, `PrismUp` installs `Prism` in its latest version.

```
user@box $ prismup
No Prism compiler installed yet.
Installation of the latest Prism compiler (0.22.0).
Downloaded prism-0.22.0-x86_64-unknown-linux-gnu.tar.gz (31.1 MB).
Set Prism version 0.22.0 as your current Prism compiler.
Please add '$HOME/.prismup/bin/' to your PATH.
```

## 5. Updating Prism

To always get and use the latest `Prism` release and set it as your current `Prism` compiler, just run `prismup --upgrade`.

```
user@box $ prismup --upgrade
The latest Prism version 0.22.0 is already installed.
Prism version 0.22.0 is already set as your current Prism compiler.
```

## 6. Setting another Prism version

### 6.1. List all Prism versions available

The versions released without a binary for your platform are marked as such.

```
user@box $ prismup --versions-list
Prism 0.22.0 (installed, current)
Prism 0.21.0 (installed)
Prism 0.20.0
Prism 0.19.0
Prism 0.18.0 (installed)
Prism 0.17.0
Prism 0.16.0
Prism 0.15.0
Prism 0.14.0
Prism 0.13.0
Prism 0.12.0
Prism 0.11.0
Prism 0.10.0
Prism 0.9.0
Prism 0.8.0
Prism 0.7.0
Prism 0.6.0
Prism 0.5.0
Prism 0.4.0 (no binary for x86_64-unknown-linux-gnu)
Prism 0.3.0 (no binary for x86_64-unknown-linux-gnu)
Prism 0.2.0 (no binary for x86_64-unknown-linux-gnu)
Prism 0.1.0 (no binary for x86_64-unknown-linux-gnu)
```

### 6.2. Setting a specific Prism version

```
user@box $ prismup --set-version 0.19.0
Prism version 0.19.0 needs to be installed before being set.
Set Prism version 0.19.0 as your current Prism compiler.
```

### 6.3. Installing a version released without a binary for your platform

```
user@box $ prismup --install-version 0.3.0
Prism version 0.3.0 is released but has no binary for x86_64-unknown-linux-gnu.
```

## 7. Full usage

```
user@box $ prismup -h
A CLI tool to install and manage versions of the Prism language.

Usage: prismup [OPTIONS]

Options:
  -v, --version                   Print PrismUp version
  -c, --current-version           Print the current Prism version
  -u, --upgrade                   Install and set the latest Prism version
  -l, --versions-list             Show list of available Prism versions
  -i, --install-version <SEMVER>  Install Prism in the given version
  -s, --set-version <SEMVER>      Set the current Prism to the given version
  -r, --remove-version <SEMVER>   Remove the given Prism version
  -f, --refresh                   Ignore the cached Prism releases and get them again from Github
  -o, --offline                   Only use the cached Prism releases and archives, without network access
  -C, --cache-clear               Clear the PrismUp cache directory contents
      --uninstall-prismup         Uninstall PrismUp
  -h, --help                      Print help
```

Only one action can be given at a time.

## 8. Cache and network

`PrismUp` caches, in `$HOME/.cache/prismup/`, the list of the `Prism` releases (for one hour) and the
archives already downloaded.  An archive whose `SHA256` integrity check passes is never downloaded
again.

```
user@box $ prismup --install-version 0.19.0
Installation of Prism compiler version 0.19.0...
Using the already downloaded archive of Prism 0.19.0.
```

Use `--refresh` to get the `Prism` releases again from `Github`, without waiting for the cache to
expire.

```
user@box $ prismup --versions-list --refresh
```

Use `--offline` to work only with the cache and without any network access.  A `Prism` version can
be installed offline only if its archive is already in the cache.

```
user@box $ prismup --install-version 0.18.0 --offline
Installation of Prism compiler version 0.18.0...
The archive of Prism 0.18.0 is not in the PrismUp cache, it cannot be installed while offline.
```

Set the `GITHUB_TOKEN` (or `GH_TOKEN`) environment variable to authenticate the requests sent to the
`Github` API, to get a higher rate limit.

## 9. Uninstall PrismUp

```
user@box $ prismup --uninstall-prismup
Removing '/home/account/.prismup'...
Removing '/home/account/.cache/prismup'...
Removing the prismup binary at '/home/account/.cargo/bin/prismup'...
You can remove '$HOME/.prismup/bin/' from your PATH.
```

