# For Windows users. If you are linux or macOS user, please delete this line.
set shell := ["powershell.exe", "-c"]

help:
    just -l

neko *args:
    cargo build --bin cli
    @echo ""
    ./target/debug/cli.exe {{args}}
    @echo ""


fmt:
    cargo +nightly  fmt --all