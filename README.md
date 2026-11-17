# VietDict Offline

Từ điển Anh ↔ Việt chạy hoàn toàn offline, viết bằng Rust.

## Tải và cài đặt

Vào [Releases](https://github.com/kingbone2006/off-dict/releases/latest).

### Windows

Tải `VietDict-windows-x64-setup.exe`, mở file và chọn **Install**. VietDict sẽ tạo shortcut trong Start Menu và Desktop; không cần cài Rust, Cargo hay chạy lệnh.

Nếu muốn dùng bản portable, tải `VietDict-windows-x64.zip`, giải nén rồi mở `vietdict.exe`.

### Linux/macOS

Tải gói `.tar.gz`, giải nén và chạy binary `vietdict`.

Bản `0.2.0` là bản GUI native đầu tiên; dữ liệu demo được đóng gói sẵn để người dùng có thể mở và tra cứu ngay.

## Hiện trạng

Đã có lõi tra cứu và công cụ build dữ liệu:

- chuẩn hoá Unicode NFC/NFD, bỏ dấu và Telex;
- exact, prefix, fuzzy, lemma (`went` → `go`, `children` → `child`);
- SQLite FTS5 tra ngược nghĩa;
- FST mmap cho gợi ý nhanh;
- lịch sử, yêu thích và cài đặt trong `user.sqlite`;
- parser OVDP/Hồ Ngọc Đức streaming.

UI Slint, CLI đầy đủ và đóng gói đa nền tảng đang được triển khai theo [AGENTS.md](AGENTS.md).

## Build

```powershell
cargo test --workspace
cargo run -p dict-builder -- parse --lang en --input data/raw/star_anhviet.tab --out data/build/en.jsonl
cargo run -p dict-builder -- parse --lang vi --input data/raw/star_vietanh.tab --out data/build/vi.jsonl
cargo run -p dict-builder -- sqlite data/build/en.jsonl data/build/vi.jsonl --out data/build
```

Dữ liệu gốc không được commit. Xem [data/LICENSES.md](data/LICENSES.md).

## Giấy phép

Mã nguồn GPL-3.0-only. Dữ liệu tuân theo giấy phép riêng của từng nguồn.
