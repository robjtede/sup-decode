set lazy := true

toolchain := ""

_list:
    @just --list

# Format project.
fmt:
    just --unstable --fmt
    # nixpkgs-fmt .
    fd --type=file --hidden --extension=md --extension=yml --exec-batch prettier --write
    fd --hidden --extension=toml --exec-batch taplo format
    cargo +nightly fmt

# Check project.
check:
    just --unstable --fmt --check
    # nixpkgs-fmt --check .
    fd --type=file --hidden --extension=md --extension=yml --exec-batch prettier --check
    fd --hidden --extension=toml --exec-batch taplo format --check
    fd --hidden --extension=toml --exec-batch taplo lint
    cargo +nightly fmt -- --check
    cargo clippy --workspace --all-targets --all-features

[private]
test-lib:
    cargo {{ toolchain }} nextest run --workspace --all-targets --all-features

[private]
test-doc:
    cargo {{ toolchain }} test --doc --workspace --all-features

[env("RUSTDOCFLAGS", "--cfg docsrs -D warnings")]
[private]
test-doc-compile:
    cargo {{ toolchain }} doc --workspace --no-deps --all-features

# Run tests.
[parallel]
test: test-lib test-doc test-doc-compile
