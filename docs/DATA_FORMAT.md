# Định dạng dữ liệu

## JSONL trung gian
Mỗi dòng là một `Record`: `headword`, `lang`, `senses`, `relations`, `inflections`, `source`.
Parser đọc streaming, không nạp file nguồn vào RAM.

## dict.sqlite
Schema 1 nằm trong `vietdict_core::schema`, gồm entry/sense/example/inflection/relation/meta và FTS5.

## FST
`en.fst` và `vi.fst` có key `headword_norm` đã bỏ dấu, value đóng gói `(frequency, entry_id)`.
