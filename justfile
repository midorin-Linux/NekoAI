# For Windows users. If you are linux or macOS user, please delete this line.
set shell := ["powershell.exe", "-c"]

help:
    just -l

neko *args:
    cargo build --bin nekoai-cli
    @echo ""
    ./target/debug/nekoai-cli.exe {{args}}
    @echo ""

qdrant-setup:
    docker pull qdrant/qdrant:latest
    docker volume create qdrant_data

qdrant-up:
    $exists = docker ps -a --filter "name=^qdrant$" -q; if ($exists) { docker start qdrant } else { docker run -d --name qdrant -p 6333:6333 -v qdrant_data:/qdrant/storage qdrant/qdrant:latest }

qdrant-down:
    $running = docker ps --filter "name=^qdrant$" -q; if ($running) { docker stop qdrant }


fmt:
    cargo +nightly  fmt --all
