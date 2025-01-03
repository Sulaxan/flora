default:
    just --list

build:
    cargo build

build_release:
    cargo build --release

run CONFIG_PATH:
    cargo run -- -c {{CONFIG_PATH}}

fmt:
    cargo +nightly fmt
