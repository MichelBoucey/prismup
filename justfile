PRISMUP_SEMVER := "0.1.0"

default:
    @just --list

lint:
    cargo clippy

fmt:
    just --fmt
    cargo fmt
    dumber -w README.md

watch:
    bacon

build:
    cargo build

install: build-release
    cargo install --path .

build-release:
    cargo build --release

compilation-test:
    prism test/optics.pr

dist: build-release
    rm -rf ~/.tmp/prismup-*
    mkdir ~/.tmp/prismup-{{ PRISMUP_SEMVER }}
    cp ReadMe.md ~/.tmp/prismup-{{ PRISMUP_SEMVER }}
    cp LICENSE ~/.tmp/prismup-{{ PRISMUP_SEMVER }}
    cp target/release/prismup ~/.tmp/prismup-{{ PRISMUP_SEMVER }}/
    cd ~/.tmp/ && tar -czvf prismup-{{ PRISMUP_SEMVER }}-linux-86_64.tar.gz prismup-{{ PRISMUP_SEMVER }}
    rm -rf ~/.tmp/prismup-{{ PRISMUP_SEMVER }}/
