# PrismUp [![CI](https://github.com/MichelBoucey/prismup/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/MichelBoucey/prismup/actions/workflows/ci.yml)

`PrismUp` is a CLI tool to install and manage versions of [Prism](https://github.com/sdiehl/prism), "an effect-typed functional language that compiles to native code through LLVM".

## 1. OS Platforms

`PrismUp` targets Unix-like OS.

## 2. Dependencies

`Prism`, as compiler, needs `LLVM/Clang` to run on your system (on `macOS`, `Prism` < 0.17.0 needs `z3` library to be installed). See the [README](https://github.com/sdiehl/prism/blob/main/README.md) of the project. `PrismUp` don't installthose dependencies but only `Prism`.

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
Installation of the latest Prism compiler (0.17.0).
Please add '$HOME/.prismup/bin/' to your PATH.
```

To always get and use the latest `Prism` release and set it as your current `Prism` compiler, just run `prismup --upgrade`.

```
user@box $ prismup --upgrade
The latest Prism version 0.17.0 is already installed.
Prism version 0.17.0 is already set as your current Prism compiler.
```

## 5. Usage

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
  -h, --help                      Print help
```

