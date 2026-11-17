# 📘 VietDict Offline — Từ điển Anh ↔ Việt viết bằng Rust

> **Tài liệu này dành cho AI Agent (và lập trình viên) thực thi dự án.**
> Đọc hết file trước khi viết dòng code đầu tiên. Làm theo đúng thứ tự các Phase, không nhảy cóc.
> Mỗi Phase phải đạt **Tiêu chí hoàn thành (Definition of Done)** rồi mới chuyển sang Phase tiếp theo.
>
> 🦀 **Ràng buộc cứng:** toàn bộ dự án viết bằng **Rust** — app, lõi tra cứu, và cả công cụ build dữ liệu.
> Giao diện **native** (không dùng Electron, không dùng webview) để app **khởi động tức thì và tốn ít RAM**.

---

## Mục lục

1. [Mục tiêu dự án](#1-mục-tiêu-dự-án)
2. [Nguyên tắc làm việc cho Agent](#2-nguyên-tắc-làm-việc-cho-agent)
3. [Phạm vi & yêu cầu](#3-phạm-vi--yêu-cầu)
4. [Công nghệ (Rust stack)](#4-công-nghệ-rust-stack)
5. [Kiến trúc hệ thống](#5-kiến-trúc-hệ-thống)
6. [Cấu trúc workspace](#6-cấu-trúc-workspace)
7. [Nguồn dữ liệu & bản quyền](#7-nguồn-dữ-liệu--bản-quyền)
8. [Thiết kế dữ liệu](#8-thiết-kế-dữ-liệu)
9. [Thuật toán tìm kiếm](#9-thuật-toán-tìm-kiếm)
10. [Bí quyết để app nhẹ & nhanh](#10-bí-quyết-để-app-nhẹ--nhanh)
11. [Quy trình thực hiện theo Phase](#11-quy-trình-thực-hiện-theo-phase)
12. [Quy ước code & Git](#12-quy-ước-code--git)
13. [Kiểm thử](#13-kiểm-thử)
14. [Hiệu năng — chỉ tiêu bắt buộc](#14-hiệu-năng--chỉ-tiêu-bắt-buộc)
15. [Đóng gói & phát hành](#15-đóng-gói--phát-hành)
16. [🚀 Đưa lên GitHub & phát hành file cài ở Releases](#16--đưa-lên-github--phát-hành-file-cài-ở-releases)
17. [💡 Gợi ý tính năng hay để thêm vào](#17--gợi-ý-tính-năng-hay-để-thêm-vào)
18. [Rủi ro thường gặp & cách xử lý](#18-rủi-ro-thường-gặp--cách-xử-lý)
19. [Checklist tổng](#19-checklist-tổng)

---

## 1. Mục tiêu dự án

Xây dựng ứng dụng từ điển **Anh → Việt** và **Việt → Anh**:

- ✅ **100% offline** — không cần Internet sau khi cài đặt.
- ✅ **Khởi động tức thì** — mở app dưới 300 ms.
- ✅ **Tra cực nhanh** — gợi ý khi đang gõ dưới 5 ms.
- ✅ **Siêu nhẹ** — file chạy dưới 15 MB, RAM dưới 60 MB.
- ✅ **Thông minh** — tra được khi gõ sai chính tả, gõ tiếng Việt không dấu, gõ dạng biến thể (`went` → `go`, `children` → `child`).
- ✅ **Đa nền tảng** — Windows / macOS / Linux trước, Android sau.

---

## 2. Nguyên tắc làm việc cho Agent

1. **Đọc trước, làm sau.** Trước mỗi Phase, đọc lại mục tương ứng.
2. **Làm nhỏ, kiểm tra liên tục.** Mỗi thay đổi phải `cargo build` được và có test đi kèm.
3. **Không tự ý đổi kiến trúc hay thêm ngôn ngữ khác.** Cần thay đổi thì ghi đề xuất vào `docs/DECISIONS.md` (định dạng ADR) kèm lý do trước.
4. **Không bao giờ gọi mạng lúc runtime.** App không được phụ thuộc crate HTTP nào (`reqwest`, `ureq`, `hyper`…). CI có bước kiểm tra điều này (mục 13).
5. **Mỗi dependency mới phải có lý do.** Kiểm tra kích thước binary sau khi thêm (`cargo bloat`). Ưu tiên crate nhỏ, ít phụ thuộc, tắt `default-features` không cần.
6. **Không `unwrap()` / `expect()` trong code app** (trừ test và trường hợp chứng minh được không thể lỗi, có comment giải thích).
7. **Tôn trọng bản quyền dữ liệu.** Chỉ dùng nguồn có giấy phép rõ ràng; ghi lại trong `data/LICENSES.md`.
8. **Ghi nhật ký tiến độ** vào `docs/PROGRESS.md` sau mỗi buổi: đã làm gì, còn gì, vướng gì.
9. **Khi không chắc chắn** — chọn phương án đơn giản hơn, ghi chú lại, tiếp tục.

---

## 3. Phạm vi & yêu cầu

### 3.1. Yêu cầu chức năng (MVP)

| Mã | Chức năng | Mô tả |
|----|-----------|-------|
| F01 | Tra Anh → Việt | Phiên âm IPA, từ loại, nghĩa, ví dụ |
| F02 | Tra Việt → Anh | Tra theo từ/cụm tiếng Việt |
| F03 | Gợi ý tức thì | Danh sách gợi ý khi gõ (prefix search) |
| F04 | Tra không dấu | Gõ `con meo` ra `con mèo` |
| F05 | Tra gần đúng | Gõ sai `recieve` vẫn gợi ý `receive` |
| F06 | Chuẩn hoá biến thể | `running`, `ran` → `run`; `mice` → `mouse` |
| F07 | Phát âm offline | TTS offline (US/UK nếu có) |
| F08 | Lịch sử tra cứu | Lưu, xoá, tìm lại |
| F09 | Từ yêu thích | Đánh dấu, phân nhóm |
| F10 | Giao diện sáng/tối | Theo hệ thống hoặc tuỳ chọn |

### 3.2. Yêu cầu phi chức năng

- Khởi động nguội < 300 ms, RAM khi chạy < 60 MB.
- File chạy (không tính dữ liệu) < 15 MB.
- Hỗ trợ **gõ tiếng Việt qua bộ gõ (IME)** như Unikey, EVKey, ibus-bamboo — bắt buộc test.
- Xử lý đúng tiếng Việt dạng **NFC** và **NFD**.
- Dùng được hoàn toàn bằng bàn phím.
- Không thu thập dữ liệu người dùng.

---

## 4. Công nghệ (Rust stack)

### 4.1. Giao diện — chọn 1, ghi vào `docs/DECISIONS.md`

| Thư viện | Ưu điểm | Nhược điểm | Khuyến nghị |
|----------|---------|------------|-------------|
| **Slint** | Native, rất nhẹ, khởi động nhanh; ngôn ngữ giao diện `.slint` dễ viết; hỗ trợ IME tốt; chạy được cả Android | Giấy phép: GPLv3 / Royalty-free (miễn phí, cần ghi công) / thương mại — đọc kỹ trước khi chọn | ⭐ **Mặc định** |
| **iced** | MIT, thuần Rust, kiến trúc Elm rõ ràng | Mobile chưa chín; IME cần kiểm tra kỹ với từng phiên bản | Phương án thay thế |
| **egui** (eframe) | Cực đơn giản, cực nhanh để làm | Giao diện "kiểu công cụ", IME tiếng Việt từng có vấn đề | Chỉ dùng cho prototype |

> ❌ Không dùng Tauri/Electron/webview — trái với mục tiêu nhẹ & nhanh.

### 4.2. Các crate chính

| Mục đích | Crate | Ghi chú |
|----------|-------|---------|
| CSDL | `rusqlite` (feature `bundled`) | SQLite + FTS5 nhúng sẵn, không phụ thuộc hệ thống |
| Gợi ý prefix & fuzzy | `fst` (feature `levenshtein`) | Chỉ mục nén cực nhỏ, mmap, tra trong micro-giây |
| Map file vào bộ nhớ | `memmap2` | Mở chỉ mục không cần đọc hết vào RAM |
| Chuẩn hoá Unicode | `unicode-normalization` | NFC/NFD, bỏ dấu |
| Đường dẫn dữ liệu | `directories` | Thư mục config/data/cache đúng chuẩn mỗi hệ điều hành |
| TTS hệ điều hành | `tts` | SAPI / AVSpeech / Speech Dispatcher |
| TTS chất lượng cao (tuỳ chọn) | `ort` (ONNX Runtime) + model **Piper** | Gói tải thêm |
| Phát âm thanh | `rodio` | |
| Phím tắt toàn cục | `global-hotkey` | Cho tính năng tra nhanh (mục 16) |
| Clipboard | `arboard` | |
| Icon khay hệ thống | `tray-icon` | |
| Lỗi | `thiserror` (thư viện), `anyhow` (binary) | |
| Log | `tracing` + `tracing-subscriber` | |
| Serialize | `serde`, `serde_json` | Chỉ dùng trong builder & settings |
| CLI | `clap` | Cho `dict-builder` và `vietdict-cli` |
| Benchmark | `criterion` | |
| Test property | `proptest` | Cho normalizer |
| (Nâng cao) Zero-copy | `rkyv` | Xem mục 10 |

---

## 5. Kiến trúc hệ thống

```
┌──────────────────────────────────────────────────────────┐
│                UI native (Slint)  — crate `app`           │
│  Ô tìm kiếm · Gợi ý · Trang chi tiết · Lịch sử · Cài đặt  │
└───────────────────────────┬──────────────────────────────┘
                            │ gọi hàm trực tiếp (cùng process, không IPC)
┌───────────────────────────▼──────────────────────────────┐
│                 crate `core` (thư viện)                   │
│ ┌──────────────┐ ┌──────────────┐ ┌───────────────────┐  │
│ │ normalize    │ │ search       │ │ lemma             │  │
│ │ (NFC, dấu,   │ │ exact/prefix │ │ (bất quy tắc +    │  │
│ │  telex)      │ │ fuzzy/FTS    │ │  quy tắc hậu tố)  │  │
│ └──────────────┘ └──────────────┘ └───────────────────┘  │
│ ┌──────────────┐ ┌──────────────┐ ┌───────────────────┐  │
│ │ dict_repo    │ │ user_repo    │ │ tts (trait)       │  │
│ │ (chỉ đọc)    │ │ (đọc/ghi)    │ │ OS / Piper        │  │
│ └──────┬───────┘ └──────┬───────┘ └───────────────────┘  │
└────────┼────────────────┼────────────────────────────────┘
         ▼                ▼
   dict.sqlite       user.sqlite        en.fst / vi.fst (mmap)
   (chỉ đọc)         (đọc/ghi)
```

**Quy tắc:**
- `core` **không phụ thuộc** vào UI → dùng lại được cho CLI, app mobile, tiện ích trình duyệt.
- Tách **dữ liệu từ điển (chỉ đọc)** và **dữ liệu người dùng (đọc/ghi)** thành 2 file. Cập nhật từ điển chỉ thay `dict.sqlite`, không mất lịch sử.
- Mọi thao tác chậm (TTS, ghi lịch sử) chạy ở **thread nền**, không bao giờ chặn thread UI.

---

## 6. Cấu trúc workspace

```
vietdict/
├── Cargo.toml                 # [workspace]
├── README.md
├── rust-toolchain.toml        # ghim phiên bản Rust
├── justfile                   # lệnh tắt: just data, just run, just release
├── docs/
│   ├── DECISIONS.md
│   ├── PROGRESS.md
│   └── DATA_FORMAT.md
├── data/
│   ├── raw/                   # dữ liệu gốc (KHÔNG commit)
│   ├── build/                 # dict.sqlite, en.fst, vi.fst (KHÔNG commit)
│   ├── lists/                 # irregular_verbs.tsv, irregular_plurals.tsv, must_have_en.txt
│   └── LICENSES.md
├── crates/
│   ├── core/                  # thư viện tra cứu
│   │   ├── src/{lib,normalize,telex,lemma,search,rank,dict_repo,user_repo,tts}.rs
│   │   ├── tests/             # normalize_cases.json, ranking_golden.json
│   │   └── benches/
│   ├── dict-builder/          # binary: biến dữ liệu gốc → dict.sqlite + *.fst
│   │   └── src/{main,parse/,merge,build_sqlite,build_fst,validate}.rs
│   ├── app/                   # binary: giao diện Slint
│   │   ├── src/main.rs
│   │   ├── ui/*.slint
│   │   └── build.rs
│   └── cli/                   # binary: `vietdict run` trên terminal
└── .github/workflows/ci.yml
```

---

## 7. Nguồn dữ liệu & bản quyền

> ⚠️ **Agent phải tự kiểm tra lại giấy phép của từng nguồn tại thời điểm tải** và ghi vào `data/LICENSES.md` (tên nguồn, URL, phiên bản, giấy phép, ngày tải, SHA-256). Không dùng dữ liệu cào từ website thương mại (Oxford, Cambridge, Laban, Soha…).

| Nguồn | Nội dung | Ghi chú giấy phép |
|-------|----------|-------------------|
| **Từ điển Hồ Ngọc Đức** (Free Vietnamese Dictionary Project) | Anh–Việt, Việt–Anh, định dạng StarDict/text | Thường phát hành GPL — kiểm tra |
| **FreeDict** (`eng-vie`, `vie-eng`) | Định dạng TEI (XML) | GPL — kiểm tra |
| **Wiktionary** (bản trích xuất JSONL của **Kaikki / wiktextract**) | IPA, từ loại, nghĩa, ví dụ, biến thể | CC BY-SA — ghi công & chia sẻ tương tự |
| **WordNet (Princeton)** | Đồng nghĩa, quan hệ ngữ nghĩa | Giấy phép WordNet (dễ dãi) |
| **CMUdict** | Phiên âm tiếng Anh (US) | BSD-style |
| **Tatoeba** | Câu ví dụ song ngữ Anh–Việt | CC BY 2.0 FR |
| **Bảng bất quy tắc** (động từ, số nhiều) | Cho lemmatizer | Tự biên soạn trong `data/lists/` |

Việc tải dữ liệu gốc làm **thủ công một lần** (hoặc bằng script shell trong `justfile`), **không** viết code tải mạng trong crate nào.

**Lưu ý copyleft:** nếu dùng dữ liệu GPL/CC BY-SA, dữ liệu phân phối kèm phải theo giấy phép đó. Hiển thị trong màn hình **Giới thiệu → Nguồn dữ liệu**.

---

## 8. Thiết kế dữ liệu

### 8.1. `dict.sqlite` (chỉ đọc)

```sql
CREATE TABLE entry (
  id            INTEGER PRIMARY KEY,
  headword      TEXT NOT NULL,          -- hiển thị: "mèo", "run"
  headword_norm TEXT NOT NULL,          -- chuẩn hoá không dấu: "meo"
  lang          TEXT NOT NULL CHECK (lang IN ('en','vi')),
  ipa_us        TEXT,
  ipa_uk        TEXT,
  frequency     INTEGER DEFAULT 0,
  level         TEXT                    -- CEFR A1..C2 (nếu có)
);
CREATE INDEX idx_entry_norm ON entry(lang, headword_norm);

CREATE TABLE sense (
  id        INTEGER PRIMARY KEY,
  entry_id  INTEGER NOT NULL REFERENCES entry(id),
  pos       TEXT,                       -- noun, verb, adj...
  gloss     TEXT NOT NULL,
  domain    TEXT,                       -- y học, tin học, kinh tế...
  ord       INTEGER NOT NULL
);
CREATE INDEX idx_sense_entry ON sense(entry_id, ord);

CREATE TABLE example (
  id        INTEGER PRIMARY KEY,
  sense_id  INTEGER NOT NULL REFERENCES sense(id),
  src_text  TEXT NOT NULL,
  dst_text  TEXT
);
CREATE INDEX idx_example_sense ON example(sense_id);

CREATE TABLE inflection (               -- went → go
  form   TEXT NOT NULL,
  lemma  TEXT NOT NULL,
  kind   TEXT,
  PRIMARY KEY (form, lemma)
) WITHOUT ROWID;

CREATE TABLE relation (
  entry_id INTEGER NOT NULL,
  target   TEXT NOT NULL,
  type     TEXT NOT NULL CHECK (type IN ('syn','ant','derived','phrasal','idiom'))
);

CREATE VIRTUAL TABLE sense_fts USING fts5(
  gloss, content='sense', content_rowid='id',
  tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);
-- ('schema_version','1'), ('data_version','2026.11.01'), ('sources','...')
```

### 8.2. `en.fst` / `vi.fst`

- Key: `headword_norm` (đã sắp xếp theo byte — yêu cầu của `fst`).
- Value: `u64` = `entry.id` (nếu nhiều mục cùng key, value trỏ tới mục có `frequency` cao nhất, mục còn lại lấy qua SQL).
- Mở bằng `memmap2` → không tốn RAM tương ứng kích thước file.

### 8.3. `user.sqlite` (đọc/ghi, chế độ WAL)

```sql
CREATE TABLE history  (headword TEXT, lang TEXT, looked_up_at INTEGER);
CREATE TABLE favorite (headword TEXT, lang TEXT, folder TEXT, note TEXT, created_at INTEGER);
CREATE TABLE srs_card (headword TEXT, lang TEXT, stability REAL, difficulty REAL,
                       due_at INTEGER, reps INTEGER, lapses INTEGER);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
```

Migration đánh số, nhúng vào binary bằng `include_str!("migrations/0001_init.sql")`, chạy lúc mở DB dựa trên `PRAGMA user_version`.

---

## 9. Thuật toán tìm kiếm

### 9.1. Chuẩn hoá (`core::normalize`)

Hàm duy nhất, dùng chung cho **cả `dict-builder` lẫn `app`** (lợi thế của việc viết toàn bộ bằng Rust: không bao giờ lệch logic giữa lúc build và lúc tra).

1. Unicode **NFC**.
2. Chữ thường.
3. Cắt khoảng trắng hai đầu, gộp khoảng trắng.
4. Bản **không dấu**: tách NFD → bỏ ký tự dấu kết hợp → đổi `đ → d`.
   - `Đường` → `duong`, `nghiêng` → `nghieng`.
5. **Telex/VNI thô** (`core::telex`): `meof` → `mèo`, `duowngf` → `đường`. Chỉ dùng như một nhánh truy vấn bổ sung.

```rust
pub fn normalize(s: &str) -> String;          // NFC + lowercase + trim
pub fn strip_diacritics(s: &str) -> String;   // "đường" -> "duong"
pub fn telex_to_vi(s: &str) -> Option<String>;
```

### 9.2. Luồng tra một truy vấn

```
input → normalize
  ├─ 1. Khớp chính xác (có dấu)              → điểm cao nhất
  ├─ 2. Khớp chính xác (không dấu) — tra FST
  ├─ 3. Lemmatize (went→go, mice→mouse, -ing/-ed/-s/-es/-ies) — bảng inflection + quy tắc
  ├─ 4. Prefix trên FST (Automaton `Str::starts_with`)
  ├─ 5. Fuzzy (`fst::automaton::Levenshtein`: ≤1 nếu ≤5 ký tự, ≤2 nếu dài hơn)
  │      — chỉ chạy khi bước 1–4 cho ít hơn N kết quả
  └─ 6. FTS5 trên nghĩa (tra ngược: "con mèo" → "cat") — chỉ khi người dùng nhấn Enter
→ gộp, loại trùng → xếp hạng → top N
```

### 9.3. Xếp hạng (`core::rank`)

```
score = 100 * exact_match
      +  60 * exact_no_diacritics
      +  50 * lemma_match
      +  30 * prefix_match
      -  15 * edit_distance
      +  10 * ln(1 + frequency)
      +   5 * in_user_history
```

Được tinh chỉnh trọng số, nhưng bộ test `ranking_golden.json` (mục 13) phải luôn xanh.

---

## 10. Bí quyết để app nhẹ & nhanh

### 10.1. Cấu hình build release

```toml
# Cargo.toml (workspace)
[profile.release]
opt-level     = 3
lto           = "fat"
codegen-units = 1
panic         = "abort"
strip         = true
debug         = false

[profile.release-small]      # bản Lite, ưu tiên kích thước
inherits  = "release"
opt-level = "z"
```

### 10.2. Khởi động tức thì

- **Hiện cửa sổ trước, nạp sau:** vẽ ô tìm kiếm ngay; mở DB/FST ở thread nền (thường chỉ vài ms nhờ mmap).
- **Không nạp toàn bộ dữ liệu vào RAM.** FST dùng `memmap2`, SQLite đọc theo trang.
- **Lazy:** TTS, OCR, model dịch chỉ khởi tạo ở lần dùng đầu tiên.
- **Không parse JSON lúc khởi động** (settings đọc từ `user.sqlite`).

### 10.3. Tra nhanh

- Mở `dict.sqlite` với URI `file:dict.sqlite?mode=ro&immutable=1`.
- `PRAGMA mmap_size = 268435456; PRAGMA cache_size = -8000; PRAGMA query_only = 1;`
- Dùng `prepare_cached` cho mọi câu truy vấn lặp lại.
- Debounce ô tìm kiếm 20–30 ms; **huỷ truy vấn cũ** nếu người dùng đã gõ ký tự mới (dùng số thứ tự truy vấn, bỏ kết quả cũ).
- Giới hạn gợi ý 20–30 dòng, dùng danh sách ảo hoá (virtualized list) của Slint (`ListView`).

### 10.4. Giữ binary nhỏ

- Tắt `default-features` của mọi crate, chỉ bật cái cần.
- Kiểm tra định kỳ: `cargo bloat --release --crates` và `cargo tree -d` (tìm crate trùng phiên bản).
- Font: chỉ nhúng **một** font hỗ trợ tiếng Việt tốt (ví dụ Be Vietnam Pro, Inter — đều SIL OFL), đã subset bằng `pyftsubset`/`hb-subset` nếu cần; hoặc dùng font hệ thống.
- (Tuỳ chọn) allocator `mimalloc` — đo trước/sau rồi mới quyết định.

### 10.5. (Nâng cao, v2) Bỏ SQLite khỏi đường tra chính

Khi cần tốc độ tối đa: serialize toàn bộ mục từ bằng **`rkyv`** thành một file nhị phân, mmap và đọc **zero-copy** (không giải mã). SQLite chỉ còn dùng cho FTS tra ngược. Chỉ làm khi benchmark chứng minh SQLite là nút thắt.

---

## 11. Quy trình thực hiện theo Phase

### 🟢 Phase 0 — Khởi tạo (0.5 ngày)

- [ ] `cargo new --lib crates/core`, `cargo new crates/dict-builder`, `crates/app`, `crates/cli`; tạo workspace.
- [ ] `rust-toolchain.toml` (stable), `rustfmt.toml`, `clippy.toml`, `.gitignore`.
- [ ] `justfile` với các lệnh: `just data`, `just run`, `just test`, `just bench`, `just release`.
- [ ] CI GitHub Actions: `fmt --check`, `clippy -D warnings`, `test` trên Windows/macOS/Linux.
- [ ] Ghi lựa chọn UI (Slint/iced) vào `docs/DECISIONS.md`.

**DoD:** `just test` chạy được; CI xanh trên 3 hệ điều hành.

---

### 🟢 Phase 1 — Công cụ build dữ liệu `dict-builder` (3–5 ngày) ⭐ *quan trọng nhất*

Chất lượng dữ liệu quyết định 80% giá trị sản phẩm.

```
dict-builder parse    --source hongocduc --input data/raw/... --out data/build/hnd.jsonl
dict-builder parse    --source wiktextract ...
dict-builder merge    data/build/*.jsonl --out data/build/merged.jsonl
dict-builder sqlite   data/build/merged.jsonl --out data/build/dict.sqlite
dict-builder fst      data/build/dict.sqlite --out data/build/
dict-builder validate data/build/dict.sqlite --report data/build/report.md
```

1. **Parse** (`parse/*.rs`, mỗi nguồn 1 module): StarDict (`.ifo/.idx/.dict`), TEI XML (`quick-xml`), JSONL (`serde_json` đọc từng dòng, **streaming**, không nạp cả file). Xuất ra JSONL chung:
   ```json
   {"headword":"run","lang":"en","ipa_us":"/rʌn/","senses":[
     {"pos":"verb","gloss":"chạy","examples":[{"src":"He runs every morning.","dst":"Anh ấy chạy mỗi sáng."}]}
   ],"source":"hongocduc"}
   ```
2. **Chuẩn hoá:** gọi đúng `core::normalize`; bỏ thẻ HTML; thống nhất từ loại (`n.`, `noun`, `danh từ` → `noun`); sửa lỗi mã hoá kiểu `Ã©` (thử giải mã lại Latin-1 → UTF-8, ghi log những dòng đã sửa).
3. **Merge:** gộp mục trùng; loại nghĩa trùng (so sánh sau chuẩn hoá + bỏ dấu câu); giữ trường `source`.
4. **SQLite:** chèn trong **một transaction**, tạo index sau khi chèn xong, build FTS, chạy `ANALYZE` và `VACUUM`.
5. **FST:** sắp xếp key theo byte, dùng `fst::MapBuilder`.
6. **Validate** — báo cáo `report.md`:
   - Số mục từ Anh/Việt, số nghĩa, số ví dụ, % có IPA.
   - Mục không có nghĩa, nghĩa rỗng, ký tự lỗi (`�`, `Ã`, `Æ`).
   - **Danh sách bắt buộc** `data/lists/must_have_en.txt` (500 từ thông dụng).
   - Kích thước file.

**DoD:**
- `just data` tạo lại toàn bộ từ đầu, kết quả **tái lập được** (chạy 2 lần cho cùng checksum).
- ≥ 99% từ trong danh sách bắt buộc có mặt; 0 lỗi mã hoá.
- `data/LICENSES.md` đầy đủ.

---

### 🟢 Phase 2 — Lõi tra cứu `core` (3–4 ngày)

API công khai:

```rust
pub struct Dictionary { /* dict.sqlite + en.fst + vi.fst */ }

impl Dictionary {
    pub fn open(data_dir: &Path) -> Result<Self, DictError>;
    pub fn suggest(&self, query: &str, lang: Lang, limit: usize) -> Vec<Suggestion>;
    pub fn lookup(&self, headword: &str, lang: Lang) -> Result<Option<Entry>, DictError>;
    pub fn reverse_lookup(&self, text: &str, lang: Lang, limit: usize) -> Result<Vec<Suggestion>, DictError>;
}

pub struct UserStore { /* user.sqlite */ }
impl UserStore {
    pub fn open(path: &Path) -> Result<Self, UserError>;  // tự chạy migration
    pub fn add_history(&self, headword: &str, lang: Lang) -> Result<(), UserError>;
    pub fn toggle_favorite(&self, headword: &str, lang: Lang) -> Result<bool, UserError>;
    // ...
}
```

- [ ] `normalize`, `telex` — qua toàn bộ `normalize_cases.json` + `proptest` (chuẩn hoá 2 lần = 1 lần).
- [ ] `lemma` — bảng bất quy tắc (nhúng bằng `include_str!`) + quy tắc hậu tố.
- [ ] `search` + `rank` — theo mục 9.
- [ ] `Dictionary` phải là `Send + Sync` để dùng chung giữa thread UI và thread nền (dùng pool kết nối nhỏ hoặc `Mutex<Connection>` mỗi thread).
- [ ] Benchmark `criterion`.

**DoD:** test xanh; đạt chỉ tiêu mục 14 trên benchmark.

---

### 🟢 Phase 3 — Giao diện MVP `app` (4–5 ngày)

Màn hình:
1. **Chính:** ô tìm kiếm luôn focus, danh sách gợi ý, nút đổi chiều Anh⇄Việt.
2. **Chi tiết:** từ, IPA, nút phát âm, nghĩa nhóm theo từ loại, ví dụ, đồng nghĩa/trái nghĩa (bấm để tra tiếp), nút ★.
3. **Lịch sử** và **Yêu thích**.
4. **Cài đặt:** sáng/tối, cỡ chữ, giọng đọc, xoá dữ liệu người dùng.
5. **Giới thiệu:** phiên bản app, phiên bản dữ liệu, nguồn & giấy phép.

Trải nghiệm:
- Phím tắt: `Ctrl/Cmd+K` focus ô tìm, `↑/↓` chọn, `Enter` mở, `Esc` xoá, `Alt+←` quay lại.
- Không có kết quả → "Có phải bạn muốn tìm: …" (từ fuzzy).
- Tra xong → ghi lịch sử ở thread nền.
- **Test IME:** gõ tiếng Việt bằng Unikey (Windows), bộ gõ macOS, ibus-bamboo (Linux).

**DoD:** dùng được hoàn toàn bằng bàn phím; IME tiếng Việt hoạt động trên 3 hệ điều hành; chạy được khi tắt mạng; khởi động < 300 ms.

---

### 🟢 Phase 4 — Phát âm offline (2 ngày)

```rust
pub trait Speaker: Send + Sync {
    fn speak(&self, text: &str, accent: Accent) -> Result<(), TtsError>;
}
```

- Mặc định: `OsSpeaker` dùng crate `tts` (giọng có sẵn của hệ điều hành, không tốn dung lượng).
- Tuỳ chọn: `PiperSpeaker` dùng `ort` + model Piper (en_US / en_GB) — **gói tải thêm**, đặt vào thư mục data.
- Cache file WAV/Opus của từ đã đọc vào thư mục cache (`directories`).

**DoD:** bấm phát âm < 300 ms (lần đầu), < 50 ms (đã cache); không chặn UI.

---

### 🟢 Phase 5 — Hoàn thiện & tối ưu (2–3 ngày)

- Đo trên máy yếu (4 GB RAM, HDD): khởi động, RAM, độ trễ gõ.
- `cargo bloat` → loại bỏ phụ thuộc thừa.
- Trường hợp biên: chuỗi rỗng, chỉ dấu cách, emoji, chuỗi rất dài, dán cả đoạn văn (chỉ lấy 64 ký tự đầu để gợi ý).
- Accessibility: độ tương phản, phóng to chữ, trình đọc màn hình (Slint có hỗ trợ AccessKit).
- `cargo audit` + `cargo deny` (kiểm tra lỗ hổng và giấy phép dependency).

---

### 🟢 Phase 6 — Đóng gói & phát hành (1–2 ngày)

Xem mục 15 (đóng gói) và mục 16 (đưa lên GitHub, tự động phát hành file cài).

**DoD:** đẩy tag `v1.0.0` → GitHub Actions tự build và đính kèm file cài của cả 3 hệ điều hành vào trang Releases; link tải trong README công khai hoạt động.

---

### 🔵 Phase 7+ — Tính năng mở rộng

Chọn từ mục 17, mỗi tính năng là một Phase nhỏ có DoD riêng.

---

## 12. Quy ước code & Git

- **Commit:** Conventional Commits — `feat:`, `fix:`, `perf:`, `data:`, `docs:`, `test:`, `refactor:`, `chore:`.
  - Ví dụ: `feat(search): hỗ trợ tra tiếng Việt không dấu`
- **Nhánh:** `main` luôn build được; làm việc trên `feat/...`, `fix/...`.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` bắt buộc trước commit.
- Lỗi: `thiserror` cho `core`, `anyhow` cho các binary.
- Không `unsafe` trừ khi có lý do và comment `// SAFETY:` giải thích (mmap là trường hợp được phép).
- Tài liệu `///` cho mọi hàm công khai trong `core`.
- Không commit dữ liệu gốc hay file build; dữ liệu phát hành đưa lên GitHub Releases.

---

## 13. Kiểm thử

| Loại | Nội dung | Công cụ |
|------|----------|---------|
| Unit | normalize, telex, lemma, rank | `cargo test` |
| Property | `normalize(normalize(x)) == normalize(x)`; NFC ≡ NFD sau chuẩn hoá | `proptest` |
| Dữ liệu | danh sách bắt buộc, lỗi mã hoá, nghĩa rỗng | `dict-builder validate` |
| Xếp hạng (golden) | truy vấn → top-3 kỳ vọng | `cargo test` + `ranking_golden.json` |
| Hiệu năng | suggest / lookup / fuzzy / khởi động | `criterion`, `hyperfine` |
| **Không có mạng** | CI fail nếu `cargo tree -p app` chứa `reqwest`, `ureq`, `hyper`, `curl` | script trong CI |
| Bảo mật & giấy phép | lỗ hổng, giấy phép dependency | `cargo audit`, `cargo deny` |

`ranking_golden.json`:
```json
[
  {"q": "went",    "lang": "en", "expect_top": ["go"]},
  {"q": "recieve", "lang": "en", "expect_top": ["receive"]},
  {"q": "children","lang": "en", "expect_top": ["child"]},
  {"q": "con meo", "lang": "vi", "expect_top": ["con mèo"]},
  {"q": "meof",    "lang": "vi", "expect_top": ["mèo"]},
  {"q": "duong",   "lang": "vi", "expect_any": ["đường", "dương", "đương"]}
]
```

---

## 14. Hiệu năng — chỉ tiêu bắt buộc

| Thao tác | Chỉ tiêu (p95, máy tầm trung) |
|----------|-------------------------------|
| Khởi động nguội đến khi gõ được | < 300 ms |
| `suggest` (prefix) | < 2 ms |
| `suggest` có fuzzy | < 15 ms |
| `lookup` chi tiết | < 5 ms |
| Tra ngược FTS | < 30 ms |
| RAM khi chạy | < 60 MB |
| File chạy `app` | < 15 MB |
| `dict.sqlite` + `*.fst` | < 100 MB |

Đo khởi động: `hyperfine --warmup 3 'vietdict --bench-startup'` (cờ ẩn: mở app, đợi sẵn sàng, ghi thời gian, thoát).

---

## 15. Đóng gói & phát hành

Dùng **`cargo-dist`** (hoặc `cargo-packager`) để tạo bộ cài tự động từ CI:

- **Windows:** `.msi` (cargo-wix) hoặc `.exe` NSIS — ký số nếu có chứng chỉ.
- **macOS:** `.dmg`, ký và notarize; build universal (x86_64 + aarch64).
- **Linux:** `.AppImage`, `.deb`; (tuỳ chọn) Flatpak.
- **Android (sau):** Slint hỗ trợ Android qua `android-activity` + `cargo-apk`/`xbuild`.
- **Bản portable:** một thư mục gồm file chạy + `data/` — giải nén là dùng, không cần cài.
- **Phiên bản:** SemVer cho app (`1.2.0`), ngày cho dữ liệu (`data-2026.11.01`).
- **Cập nhật dữ liệu:** app kiểm tra `meta.schema_version`; người dùng tự tải gói dữ liệu mới và nhập vào app (vẫn giữ nguyên tắc không tự gọi mạng).
- Viết `CHANGELOG.md` mỗi lần phát hành.

---

## 16. 🚀 Đưa lên GitHub & phát hành file cài ở Releases

### 16.1. Hai loại tài liệu — đừng nhầm

| File | Dành cho | Nội dung |
|------|----------|----------|
| `AGENTS.md` | AI Agent & lập trình viên | **Chính là tài liệu bạn đang đọc.** Trước khi push lần đầu, **đổi tên file này thành `AGENTS.md`** |
| `README.md` | Người dùng cuối & khách xem repo | Giới thiệu, ảnh chụp màn hình, link tải, cách dùng — dùng mẫu ở mục 16.4 |

> Nhiều công cụ AI coding tự đọc `AGENTS.md` ở thư mục gốc, nên đặt tên này giúp agent luôn có ngữ cảnh.

### 16.2. Chuẩn bị (làm một lần)

```bash
# Cài GitHub CLI: https://cli.github.com
gh auth login                       # chọn GitHub.com → HTTPS → đăng nhập bằng trình duyệt
git config --global user.name  "Tên của bạn"
git config --global user.email "email@example.com"
git config --global init.defaultBranch main
```

### 16.3. Các file bắt buộc có trước khi push

```
vietdict/
├── README.md                  # README công khai (mục 16.4)
├── AGENTS.md                  # tài liệu này
├── LICENSE                    # MIT / Apache-2.0 / GPL-3.0 (xem mục Giấy phép)
├── CHANGELOG.md               # theo chuẩn "Keep a Changelog"
├── CONTRIBUTING.md            # hướng dẫn đóng góp, quy ước commit
├── .gitignore
├── assets/
│   ├── icon.png               # icon app 512×512
│   ├── icon.ico               # Windows
│   ├── icon.icns              # macOS
│   └── screenshots/*.png      # ảnh chụp màn hình cho README
└── .github/
    ├── workflows/
    │   ├── ci.yml             # chạy mỗi lần push / pull request
    │   └── release.yml        # chạy khi đẩy tag v*
    ├── ISSUE_TEMPLATE/
    │   ├── bug_report.md
    │   └── feature_request.md
    └── pull_request_template.md
```

`.gitignore` tối thiểu:

```gitignore
/target
/data/raw/
/data/build/
/dist/
*.sqlite-wal
*.sqlite-shm
.DS_Store
.env
*.pem
*.p12
```

> 🔒 **Tuyệt đối không commit** token, mật khẩu, chứng chỉ ký số (`.pfx`, `.p12`), file `.env`. Mọi bí mật để trong **GitHub → Settings → Secrets and variables → Actions**.

### 16.4. Mẫu `README.md` công khai

Thay `<username>` bằng tên tài khoản GitHub của bạn.

````markdown
<div align="center">
  <img src="assets/icon.png" width="96" alt="VietDict">
  <h1>VietDict</h1>
  <p><b>Từ điển Anh ↔ Việt offline — nhẹ, nhanh, miễn phí</b></p>

  [![Release](https://img.shields.io/github/v/release/<username>/vietdict)](https://github.com/<username>/vietdict/releases/latest)
  [![CI](https://github.com/<username>/vietdict/actions/workflows/ci.yml/badge.svg)](https://github.com/<username>/vietdict/actions/workflows/ci.yml)
  [![Downloads](https://img.shields.io/github/downloads/<username>/vietdict/total)](https://github.com/<username>/vietdict/releases)
  [![License](https://img.shields.io/github/license/<username>/vietdict)](LICENSE)
</div>

![Ảnh chụp màn hình](assets/screenshots/main.png)

## ✨ Tính năng
- 🔌 Chạy hoàn toàn **offline**, không thu thập dữ liệu
- ⚡ Mở app dưới 0,3 giây, gợi ý ngay khi gõ
- 🔤 Gõ không dấu (`con meo` → *con mèo*), gõ sai vẫn tìm ra (`recieve` → *receive*)
- 🔁 Hiểu dạng biến thể: `went` → *go*, `children` → *child*
- 🔊 Phát âm offline
- ⭐ Lịch sử, từ yêu thích, giao diện sáng/tối

## 📥 Tải về

| Hệ điều hành | File cài |
|--------------|----------|
| Windows 10/11 (64-bit) | [VietDict-windows-x64.msi](https://github.com/<username>/vietdict/releases/latest/download/VietDict-windows-x64.msi) · [bản portable .zip](https://github.com/<username>/vietdict/releases/latest/download/VietDict-windows-x64-portable.zip) |
| macOS 11+ (Intel & Apple Silicon) | [VietDict-macos-universal.dmg](https://github.com/<username>/vietdict/releases/latest/download/VietDict-macos-universal.dmg) |
| Linux (Ubuntu/Debian) | [VietDict-linux-x64.deb](https://github.com/<username>/vietdict/releases/latest/download/VietDict-linux-x64.deb) · [.tar.gz](https://github.com/<username>/vietdict/releases/latest/download/VietDict-linux-x64.tar.gz) |

Xem tất cả phiên bản tại trang [Releases](https://github.com/<username>/vietdict/releases).

> **Windows báo "Windows protected your PC"?** Bấm **More info → Run anyway** (do bản phát hành chưa ký số).
> **macOS báo "không thể mở"?** Chuột phải vào app → **Open**.

## ⌨️ Phím tắt
| Phím | Chức năng |
|------|-----------|
| `Ctrl/Cmd + K` | Đưa con trỏ vào ô tìm kiếm |
| `↑ / ↓`, `Enter` | Chọn và mở từ |
| `Esc` | Xoá ô tìm kiếm |

## 🛠️ Tự build từ mã nguồn
```bash
git clone https://github.com/<username>/vietdict.git
cd vietdict
cargo install just
just data        # build dữ liệu từ điển (xem AGENTS.md mục 7)
just run
```

## 📚 Nguồn dữ liệu
Xem [data/LICENSES.md](data/LICENSES.md).

## 📄 Giấy phép
Mã nguồn: xem [LICENSE](LICENSE). Dữ liệu từ điển theo giấy phép của từng nguồn.
````

> 💡 **Mẹo quan trọng:** đặt **tên file cài cố định, không chứa số phiên bản** (ví dụ `VietDict-windows-x64.msi`). Khi đó link dạng `/releases/latest/download/<tên-file>` **luôn trỏ tới bản mới nhất**, không bao giờ phải sửa README.

### 16.5. Tạo repo và push lần đầu

```bash
cd vietdict
git mv README.md AGENTS.md          # tài liệu agent
# ... tạo README.md công khai theo mẫu 16.4 ...
git init
git add .
git commit -m "chore: khởi tạo dự án VietDict"

# Tạo repo trên GitHub và push luôn
gh repo create vietdict --public --source=. --remote=origin --push \
  --description "Từ điển Anh ↔ Việt offline viết bằng Rust"

# Thêm chủ đề để dễ tìm
gh repo edit --add-topic rust,dictionary,vietnamese,english,offline,slint
```

### 16.6. Bảo vệ nhánh `main`

GitHub → **Settings → Branches → Add rule** cho `main`:
- ✅ Require a pull request before merging
- ✅ Require status checks to pass (chọn job `ci`)
- ✅ Không cho force-push

Từ đây mọi thay đổi đi theo luồng: `git switch -c feat/...` → commit → `gh pr create` → CI xanh → merge.

### 16.7. Phát hành gói dữ liệu (tách khỏi app)

Dữ liệu gốc không được commit và thay đổi chậm hơn code, nên build **trên máy local** rồi đưa lên một release riêng. Workflow phát hành app sẽ tải gói này về để đóng vào file cài.

```bash
just data                                       # tạo data/build/dict.sqlite, en.fst, vi.fst
cd data/build
zip -9 vietdict-data.zip dict.sqlite en.fst vi.fst ../LICENSES.md
sha256sum vietdict-data.zip > vietdict-data.zip.sha256

gh release create data-2026.11.01 vietdict-data.zip vietdict-data.zip.sha256 \
  --title "Dữ liệu từ điển 2026.11.01" \
  --notes "Gói dữ liệu dùng cho các bản phát hành app. Xem LICENSES.md bên trong." \
  --prerelease
```

Ghi phiên bản dữ liệu đang dùng vào file `data/DATA_VERSION` (một dòng, ví dụ `data-2026.11.01`) và commit file này.

### 16.8. Workflow phát hành tự động `.github/workflows/release.yml`

Kích hoạt khi đẩy tag `v*`. Build trên 3 hệ điều hành, đóng gói, tạo **bản nháp release** kèm file cài và `SHA256SUMS.txt`.

```yaml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.name }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - name: windows-x64
            os: windows-latest
            target: x86_64-pc-windows-msvc
          - name: macos-universal
            os: macos-latest
            target: aarch64-apple-darwin
            target2: x86_64-apple-darwin
          - name: linux-x64
            os: ubuntu-22.04
            target: x86_64-unknown-linux-gnu

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}${{ matrix.target2 && format(',{0}', matrix.target2) || '' }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Cài thư viện hệ thống (Linux)
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libfontconfig1-dev libxkbcommon-dev \
            libwayland-dev libxcb1-dev libspeechd-dev libclang-dev
          cargo install cargo-deb --locked

      - name: Cài cargo-wix (Windows)
        if: runner.os == 'Windows'
        run: cargo install cargo-wix --locked

      - name: Tải gói dữ liệu từ điển
        shell: bash
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          DATA_TAG=$(cat data/DATA_VERSION)
          mkdir -p data/build
          gh release download "$DATA_TAG" -p "vietdict-data.zip*" -D data/build
          cd data/build
          if command -v sha256sum >/dev/null; then sha256sum -c vietdict-data.zip.sha256
          else shasum -a 256 -c vietdict-data.zip.sha256; fi
          unzip -o vietdict-data.zip

      - name: Build
        shell: bash
        run: |
          cargo build --release -p app --target ${{ matrix.target }}
          if [ -n "${{ matrix.target2 }}" ]; then
            cargo build --release -p app --target ${{ matrix.target2 }}
          fi

      - name: Đóng gói Windows (.msi + portable .zip)
        if: runner.os == 'Windows'
        shell: bash
        run: |
          mkdir -p dist/portable/data
          cp target/${{ matrix.target }}/release/vietdict.exe dist/portable/
          cp data/build/{dict.sqlite,en.fst,vi.fst} data/LICENSES.md dist/portable/data/
          (cd dist/portable && 7z a ../VietDict-windows-x64-portable.zip .)
          cargo wix -p app --no-build --nocapture \
            --target ${{ matrix.target }} \
            --output dist/VietDict-windows-x64.msi

      - name: Đóng gói macOS (universal .dmg)
        if: runner.os == 'macOS'
        run: |
          APP=dist/VietDict.app
          mkdir -p $APP/Contents/MacOS $APP/Contents/Resources/data
          lipo -create \
            target/aarch64-apple-darwin/release/vietdict \
            target/x86_64-apple-darwin/release/vietdict \
            -output $APP/Contents/MacOS/vietdict
          cp packaging/macos/Info.plist $APP/Contents/
          cp assets/icon.icns $APP/Contents/Resources/
          cp data/build/{dict.sqlite,en.fst,vi.fst} data/LICENSES.md $APP/Contents/Resources/data/
          hdiutil create -volname VietDict -srcfolder dist/VietDict.app \
            -ov -format UDZO dist/VietDict-macos-universal.dmg

      - name: Đóng gói Linux (.deb + .tar.gz)
        if: runner.os == 'Linux'
        run: |
          mkdir -p dist/VietDict/data
          cp target/${{ matrix.target }}/release/vietdict dist/VietDict/
          cp data/build/{dict.sqlite,en.fst,vi.fst} data/LICENSES.md dist/VietDict/data/
          tar -C dist -czf dist/VietDict-linux-x64.tar.gz VietDict
          cargo deb -p app --no-build --target ${{ matrix.target }} \
            --output dist/VietDict-linux-x64.deb

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.name }}
          path: |
            dist/*.msi
            dist/*.zip
            dist/*.dmg
            dist/*.deb
            dist/*.tar.gz

  release:
    name: Tạo GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Tạo checksum
        run: cd dist && sha256sum * > SHA256SUMS.txt

      - name: Lấy ghi chú phát hành từ CHANGELOG
        run: |
          VERSION=${GITHUB_REF_NAME#v}
          awk "/^## \\[$VERSION\\]/{flag=1;next}/^## \\[/{flag=0}flag" CHANGELOG.md > NOTES.md
          [ -s NOTES.md ] || echo "Xem CHANGELOG.md" > NOTES.md

      - uses: softprops/action-gh-release@v2
        with:
          draft: true                 # xem lại rồi mới bấm Publish
          name: VietDict ${{ github.ref_name }}
          body_path: NOTES.md
          files: dist/*
```

**Cấu hình cần có trong `crates/app/Cargo.toml`** để `cargo-deb` và `cargo-wix` hoạt động:

```toml
[package]
name        = "app"
version     = "1.0.0"
description = "Từ điển Anh ↔ Việt offline"
license     = "MIT"
repository  = "https://github.com/<username>/vietdict"

[[bin]]
name = "vietdict"
path = "src/main.rs"

[package.metadata.deb]
name    = "vietdict"
section = "education"
assets  = [
  ["target/release/vietdict",       "usr/bin/",                         "755"],
  ["../../data/build/dict.sqlite",  "usr/share/vietdict/data/",         "644"],
  ["../../data/build/en.fst",       "usr/share/vietdict/data/",         "644"],
  ["../../data/build/vi.fst",       "usr/share/vietdict/data/",         "644"],
  ["../../packaging/linux/vietdict.desktop", "usr/share/applications/", "644"],
  ["../../assets/icon.png",         "usr/share/icons/hicolor/512x512/apps/vietdict.png", "644"],
]
```

- Chạy `cargo wix init -p app` một lần để sinh `crates/app/wix/main.wxs`, sau đó sửa để chép thư mục `data/` vào cùng thư mục cài.
- Tạo `packaging/macos/Info.plist` và `packaging/linux/vietdict.desktop`.
- App tìm dữ liệu theo thứ tự: biến môi trường `VIETDICT_DATA` → thư mục `data/` cạnh file chạy (portable) → `Contents/Resources/data` (macOS) → `/usr/share/vietdict/data` (Linux).

> Agent **phải chạy thử workflow** trước khi phát hành thật: đẩy tag thử `v0.0.1-test`, kiểm tra cả 5 file cài, sau đó xoá bằng `gh release delete v0.0.1-test --cleanup-tag`.

### 16.9. Quy trình phát hành một phiên bản (mỗi lần ra bản mới)

```bash
# 1. Đảm bảo main sạch và CI xanh
git switch main && git pull
cargo test --workspace && cargo clippy --workspace -- -D warnings

# 2. Tăng version trong Cargo.toml (dùng cargo-release hoặc sửa tay)
cargo install cargo-release --locked
cargo release version 1.1.0 --execute --no-confirm

# 3. Cập nhật CHANGELOG.md: chuyển mục [Unreleased] thành [1.1.0] - 2026-11-20
#    (có thể tự sinh từ commit bằng git-cliff: `git cliff --tag v1.1.0 -o CHANGELOG.md`)

# 4. Commit, gắn tag, đẩy lên
git commit -am "chore(release): v1.1.0"
git tag -a v1.1.0 -m "VietDict v1.1.0"
git push origin main --follow-tags
```

5. Vào tab **Actions** theo dõi workflow `Release` (khoảng 10–20 phút).
6. Vào **Releases** → mở bản nháp → **tải thử từng file cài và cài trên máy thật** → kiểm tra checksum.
7. Bấm **Publish release**. Link `latest/download/...` trong README tự trỏ sang bản mới.

**Đánh số phiên bản (SemVer):**
- `1.0.1` — sửa lỗi
- `1.1.0` — thêm tính năng, không phá vỡ dữ liệu cũ
- `2.0.0` — đổi `schema_version` của dữ liệu hoặc thay đổi lớn
- `1.1.0-beta.1` — bản thử nghiệm (đánh dấu **pre-release**)

### 16.10. Ký số (khuyến nghị khi có người dùng thật)

Bản chưa ký vẫn chạy được, nhưng Windows SmartScreen và macOS Gatekeeper sẽ cảnh báo.

| Nền tảng | Cách làm | Secret cần thêm |
|----------|----------|-----------------|
| Windows | Chứng chỉ code signing (OV/EV) hoặc dịch vụ ký trên cloud (Azure Trusted Signing…), ký bằng `signtool` sau bước `cargo wix` | `WINDOWS_CERT_BASE64`, `WINDOWS_CERT_PASSWORD` |
| macOS | Tài khoản Apple Developer → ký `codesign --options runtime` → notarize bằng `xcrun notarytool` → `xcrun stapler staple` | `APPLE_CERT_BASE64`, `APPLE_CERT_PASSWORD`, `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD` |
| Linux | Không bắt buộc; có thể ký checksum bằng GPG hoặc `minisign` | `MINISIGN_KEY` |

> Có thể dùng **`cargo-dist`** để sinh tự động phần lớn workflow trên (`cargo dist init`). Nếu dùng, ghi vào `docs/DECISIONS.md` và vẫn giữ quy ước tên file cố định.

### 16.11. Chăm sóc repo sau khi phát hành

- Bật **Dependabot** (`.github/dependabot.yml`) cho `cargo` và `github-actions`, hàng tuần.
- Bật **GitHub Discussions** để người dùng hỏi đáp, dành **Issues** cho lỗi và đề xuất.
- Ghim một issue "Lộ trình phát triển" (chép từ mục 17).
- Thêm ảnh chụp màn hình/GIF mới vào README mỗi khi giao diện thay đổi đáng kể.
- (Tuỳ chọn) Đưa lên kho cài đặt: **winget** (`wingetcreate`), **Homebrew Cask**, **Flathub**, **AUR**.

---

## 17. 💡 Gợi ý tính năng hay để thêm vào

Xếp theo **giá trị / công sức** (⭐ càng nhiều càng nên làm sớm). Cột crate là gợi ý Rust tương ứng.

### 🔥 Nhóm "đáng làm nhất"

| Tính năng | Mô tả | Crate | Giá trị |
|-----------|-------|-------|---------|
| **Tra nhanh bằng phím tắt toàn cục** | Bôi đen chữ ở app bất kỳ → `Ctrl+Alt+D` → popup nhỏ hiện nghĩa cạnh con trỏ. App chạy nền ở khay hệ thống, tốn RAM rất ít nhờ Rust | `global-hotkey`, `arboard`, `tray-icon` | ⭐⭐⭐⭐⭐ |
| **Theo dõi clipboard** (bật/tắt) | Copy một từ tiếng Anh → tự hiện nghĩa | `arboard` | ⭐⭐⭐⭐ |
| **Flashcard lặp lại ngắt quãng** | Thuật toán **FSRS** (hiện đại hơn SM-2); tự tạo thẻ từ yêu thích/lịch sử | `fsrs` | ⭐⭐⭐⭐⭐ |
| **Gõ Telex/VNI không cần bộ gõ** | `meof` → `mèo` — tiện khi máy chưa bật Unikey | tự viết trong `core::telex` | ⭐⭐⭐⭐ |
| **Nhãn CEFR / IELTS / TOEIC** | Gắn A1–C2, lọc danh sách học (tự biên soạn, không sao chép danh sách có bản quyền) | — | ⭐⭐⭐⭐ |
| **CLI** | `vietdict run` in nghĩa ngay trên terminal — gần như miễn phí vì `core` đã tách riêng | `clap` | ⭐⭐⭐⭐ |

### 🧠 Nhóm "thông minh"

| Tính năng | Mô tả | Crate |
|-----------|-------|-------|
| **Dịch câu/đoạn offline** | Model dịch nhỏ **OPUS-MT (Helsinki-NLP) en↔vi**, lượng tử hoá INT8, chạy local. Gói tải thêm (~100–300 MB) | `ct2rs` (CTranslate2) hoặc `ort` |
| **Quét màn hình để tra (OCR)** | Chọn vùng màn hình → nhận chữ → tra | `ocrs` (OCR thuần Rust), `xcap` (chụp màn hình) |
| **Tra trong PDF/ePub** | Mở sách, bấm vào từ là ra nghĩa, tự lưu từ mới | `pdfium-render`, `epub` |
| **Phân tích từ phái sinh** | `unhappiness` → `un-` + `happy` + `-ness` | tự viết |
| **Họ từ** | `decide → decision → decisive → decisively` | dữ liệu `relation` |
| **Phrasal verbs & idioms** | Tách riêng give up, look after, break the ice… | dữ liệu `relation` |
| **Collocation** | `make a decision`, `heavy rain` | dữ liệu |
| **Chấm phát âm offline** | Người dùng đọc thử, so với từ chuẩn | `whisper-rs` (whisper.cpp tiny) hoặc `vosk` |
| **Cặp từ dễ nhầm** | `affect/effect`, `lose/loose`, `economic/economical` | dữ liệu |

### 🎯 Nhóm "giữ chân người dùng"

| Tính năng | Mô tả |
|-----------|-------|
| **Từ của ngày** | Thông báo mỗi sáng theo cấp độ người dùng (`notify-rust`) |
| **Thống kê học tập** | Chuỗi ngày học, số từ đã thuộc, biểu đồ tiến độ |
| **Mini game** | Ghép từ–nghĩa, điền chữ thiếu, nghe và gõ lại |
| **Sổ tay cá nhân** | Ghi chú riêng, câu ví dụ của mình cho từng từ |
| **Xuất/nhập** | Xuất sang **Anki** (CSV/`.apkg`), sao lưu/khôi phục `user.sqlite` |

### 🛠️ Nhóm "kỹ thuật hay"

| Tính năng | Mô tả |
|-----------|-------|
| **Nhập từ điển ngoài** | Đọc StarDict / DSL / MDict → từ điển chuyên ngành y, luật, IT. `dict-builder` đã có parser StarDict, chỉ cần đưa vào app |
| **Đồng bộ qua mạng LAN** | Đồng bộ lịch sử/yêu thích giữa điện thoại và máy tính trong cùng Wi-Fi, ghép đôi bằng mã QR — không cloud, không server (`mdns-sd`, `qrcode`) |
| **Bản Lite** | 30.000 từ thông dụng, build `release-small`, tổng < 20 MB cho máy yếu |
| **Tiện ích trình duyệt** | Extension gọi app local qua *native messaging* (stdin/stdout JSON) — tra từ trên web không cần mạng |
| **Chạy trên web (WASM)** | Biên dịch `core` sang WebAssembly làm bản demo/PWA dùng offline |

### 🗺️ Lộ trình đề xuất

```
v1.0  MVP (Phase 0–6)
v1.1  Phím tắt toàn cục + popup + khay hệ thống + clipboard + Telex + CLI
v1.2  Flashcard FSRS + từ của ngày + thống kê
v1.3  Nhãn CEFR/IELTS + họ từ + phrasal verbs/idioms
v2.0  Dịch câu offline (OPUS-MT) + OCR (ocrs)
v2.1  Nhập StarDict/MDict + đồng bộ LAN + tối ưu rkyv
v2.2  Android
```

---

## 18. Rủi ro thường gặp & cách xử lý

| Rủi ro | Cách xử lý |
|--------|------------|
| IME tiếng Việt lỗi trong UI (mất chữ, nhân đôi dấu) | Test sớm ngay từ Phase 3 trên cả 3 hệ điều hành; nếu framework lỗi, ghi ADR và cân nhắc đổi Slint ↔ iced |
| NFC/NFD lẫn lộn → tra không ra | Mọi chuỗi đều qua `core::normalize`; có property test |
| Dữ liệu nguồn lỗi mã hoá | Sửa trong bước parse, ghi log, báo cáo trong `validate` |
| Nghĩa trùng giữa các nguồn | So sánh sau chuẩn hoá + bỏ dấu câu; ưu tiên nguồn chất lượng cao |
| Binary phình to | `cargo bloat`, tắt default-features, tách tính năng nặng thành gói tuỳ chọn |
| Fuzzy chậm | Levenshtein automaton trên FST, chỉ chạy khi thiếu kết quả |
| SQLite bị khoá khi nhiều thread | `dict.sqlite` chỉ đọc + `immutable=1`; `user.sqlite` dùng WAL, ghi qua một thread nền duy nhất |
| Thời gian biên dịch lâu | Dùng `sccache`; trong dev bật `opt-level = 1` cho dependency |
| Vi phạm giấy phép (dữ liệu hoặc crate) | `data/LICENSES.md` + `cargo deny check licenses` |
| Giấy phép Slint không phù hợp | Đọc kỹ 3 lựa chọn giấy phép của Slint ở Phase 0; nếu không hợp, dùng iced (MIT) |
| Lộ token/chứng chỉ lên GitHub | `.gitignore` chặn sẵn; chỉ dùng GitHub Secrets; nếu lỡ lộ → thu hồi ngay, không chỉ xoá commit |
| Workflow release lỗi đúng ngày phát hành | Luôn chạy thử bằng tag `v0.0.1-test`; release ở chế độ nháp |

---

## 19. Checklist tổng

- [ ] Phase 0 — Workspace, CI xanh trên 3 hệ điều hành
- [ ] Phase 1 — `just data` tái lập được, báo cáo sạch, đủ giấy phép
- [ ] Phase 2 — `core` đạt chỉ tiêu hiệu năng
- [ ] Phase 3 — UI native, IME tiếng Việt ổn, khởi động < 300 ms, chạy khi tắt mạng
- [ ] Phase 4 — Phát âm offline không chặn UI
- [ ] Phase 5 — Tối ưu, `cargo audit`/`cargo deny` sạch
- [ ] Phase 6 — Bộ cài Windows/macOS/Linux + bản portable
- [ ] Repo GitHub: README công khai, AGENTS.md, LICENSE, CHANGELOG, bảo vệ nhánh `main`
- [ ] Gói dữ liệu đã đưa lên release `data-YYYY.MM.DD`
- [ ] Đẩy tag `v*` → Actions tự build → Release có đủ `.msi`, `.zip`, `.dmg`, `.deb`, `.tar.gz`, `SHA256SUMS.txt`
- [ ] Link tải `releases/latest/download/...` trong README hoạt động
- [ ] `docs/PROGRESS.md`, `docs/DECISIONS.md`, `CHANGELOG.md` cập nhật
- [ ] Màn hình "Giới thiệu" hiển thị đủ nguồn dữ liệu & giấy phép

---

## Giấy phép

- **Mã nguồn:** đề xuất MIT hoặc Apache-2.0. Nếu dữ liệu đi kèm là GPL, hoặc dùng Slint theo GPLv3, cân nhắc phát hành mã nguồn theo **GPL-3.0** để tránh xung đột.
- **Dữ liệu:** theo giấy phép từng nguồn — xem `data/LICENSES.md`.

---

*Nhớ: dữ liệu tốt > tốc độ > giao diện đẹp > tính năng nhiều.* 🦀🚀
