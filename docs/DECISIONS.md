# Quyết định kiến trúc

## ADR-001 — Workspace Rust thuần
- **Trạng thái:** accepted
- Toàn bộ app, lõi tra cứu và builder đều là Rust workspace; không runtime HTTP.

## ADR-002 — Slint cho UI native
- **Trạng thái:** accepted
- Chọn Slint vì native, hỗ trợ Windows/macOS/Linux/Android và IME tốt.
- Phiên bản hiện tại: 1.19; bật `compat-1-2`, renderer software.
- Slint dùng giấy phép kép GPLv3 / Royalty-free / thương mại; phải rà soát trước phát hành thương mại.

## ADR-003 — OVDP làm nguồn MVP
- **Trạng thái:** accepted
- Dùng file StarDict/OVDP đã tải thủ công, parser chỉ đọc file local.
- Không tải dữ liệu ở runtime.

## ADR-004 — SQLite + FST
- `dict.sqlite` chỉ đọc, FST mmap; `user.sqlite` WAL riêng.
- Build deterministic: record đầu vào phải được merge/sort ổn định trước khi phát hành.

## ADR-005 — Tần suất và xếp hạng
- FrequencyWords chỉ làm dữ liệu phụ trợ, không bắt buộc để chạy lõi.
- Dữ liệu corpus dùng trọng số `4 * ln(1 + frequency)` để không lấn át exact/lemma.
