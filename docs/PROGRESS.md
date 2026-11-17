# Tiến độ

## 2026-11-17
- Khởi tạo workspace Rust 4 crate.
- Chọn Slint và ghi ADR.
- Hoàn thành `core`: normalize NFC/NFD, bỏ dấu, Telex, lemma, rank, schema, SQLite/FST repository, FTS reverse lookup, user store WAL/migration, TTS trait.
- Hoàn thành `dict-builder` parser OVDP streaming và build SQLite/FST cơ bản.
- `cargo test -p vietdict-core`: 20 tests pass.
- `cargo test -p dict-builder`: 4 tests pass.
- Còn lại: merge/validate đầy đủ, UI Slint MVP, CLI, CI/release.
