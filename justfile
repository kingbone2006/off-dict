set shell := ["powershell.exe", "-NoProfile", "-Command"]

test:
    cargo test --workspace

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

parse-en input out:
    cargo run -p dict-builder -- parse --lang en --input {{input}} --out {{out}}

parse-vi input out:
    cargo run -p dict-builder -- parse --lang vi --input {{input}} --out {{out}}

run:
    cargo run -p app
