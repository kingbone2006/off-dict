//! Parser định dạng `.tab` của OVDP / Từ điển Hồ Ngọc Đức (catusf/tudien).
//!
//! Mỗi dòng: `headword<TAB>body`. Trong `body`, các ký hiệu đứng sau khoảng trắng:
//! `*` từ loại, `-` nghĩa, `=` ví dụ (`câu+dịch`), `!` thành ngữ, `@` lĩnh vực.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use vietdict_core::normalize::normalize;

use crate::clean::{canonical_pos, fix_mojibake, fix_stray_capital_a_tilde, strip_html, tidy};
use crate::record::{ExampleRec, InflectionRec, Record, SenseRec};

/// Thống kê khi parse.
#[derive(Debug, Default)]
pub struct Stats {
    pub lines: usize,
    pub records: usize,
    pub skipped: usize,
    pub fixed_mojibake: usize,
    pub fixed_stray: usize,
}

fn is_marker(c: char) -> bool {
    matches!(c, '*' | '-' | '=' | '!' | '@')
}

/// Tách body thành các cặp (ký hiệu, nội dung). Ký hiệu `'\0'` = phần đầu không có ký hiệu.
fn tokenize(body: &str) -> Vec<(char, String)> {
    let mut out = Vec::new();
    let mut cur_marker = '\0';
    let mut cur = String::new();
    let mut prev_ws = true;
    for c in body.chars() {
        if prev_ws && is_marker(c) {
            out.push((cur_marker, std::mem::take(&mut cur)));
            cur_marker = c;
        } else {
            cur.push(c);
        }
        prev_ws = c.is_whitespace();
    }
    out.push((cur_marker, cur));
    out.into_iter()
        .map(|(m, t)| (m, tidy(&t)))
        .filter(|(m, t)| *m != '\0' || !t.is_empty())
        .collect()
}

fn domain_of(text: &str) -> Option<String> {
    let t = text.trim();
    let t = t
        .strip_prefix("Chuyên ngành")
        .or_else(|| t.strip_prefix("Lĩnh vực:"))
        .or_else(|| t.strip_prefix("Lĩnh vực"))
        .unwrap_or(t)
        .trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Rác kiểu `mauz/` (mảnh phiên âm bị cắt).
fn is_junk_gloss(g: &str) -> bool {
    g.is_empty() || (!g.contains(' ') && g.ends_with('/') && g.len() < 20)
}

fn is_word(s: &str) -> bool {
    !s.is_empty() && s.len() <= 24 && s.chars().all(|c| c.is_ascii_alphabetic() || c == '\'')
}

/// Trích biến thể từ nhãn từ loại: `nội động từ (went, gone)`, `danh từ, số nhiều mice`.
fn inflections_from_pos(head: &str, pos_text: &str, out: &mut Vec<InflectionRec>) {
    let head_n = normalize(head);
    let mut push = |form: &str, kind: &str| {
        let f = normalize(form);
        if is_word(&f) && f != head_n && !out.iter().any(|i| i.form == f) {
            out.push(InflectionRec {
                form: f,
                lemma: head_n.clone(),
                kind: kind.to_string(),
            });
        }
    };
    if let Some(i) = pos_text.find("số nhiều") {
        let rest = &pos_text[i + "số nhiều".len()..];
        if let Some(w) = rest.split_whitespace().next() {
            push(w.trim_matches(|c: char| !c.is_ascii_alphabetic()), "plural");
        }
    }
    if let (Some(a), Some(b)) = (pos_text.find('('), pos_text.find(')')) {
        if a < b {
            let parts: Vec<&str> = pos_text[a + 1..b].split(',').map(str::trim).collect();
            if parts.len() >= 2 && parts.iter().all(|p| is_word(p)) {
                for p in parts {
                    push(p, "past");
                }
            }
        }
    }
}

/// Parse một file `.tab` mà không nạp file vào RAM.
pub fn parse_file(path: &Path, lang: &str) -> Result<(Vec<Record>, Stats)> {
    let file = File::open(path).with_context(|| format!("mở {}", path.display()))?;
    let mut stats = Stats::default();
    let mut records = Vec::new();
    for line in BufReader::new(file).lines() {
        stats.lines += 1;
        let line = line?;
        let Some((raw_head, raw_body)) = line.split_once('\t') else {
            stats.skipped += 1;
            continue;
        };
        let mut body = raw_body.to_string();
        if let Some(x) = fix_mojibake(&body) {
            body = x;
            stats.fixed_mojibake += 1;
        }
        if let Some(x) = fix_stray_capital_a_tilde(&body) {
            body = x;
            stats.fixed_stray += 1;
        }
        let headword = tidy(&strip_html(raw_head));
        if headword.is_empty() {
            stats.skipped += 1;
            continue;
        }
        let mut senses = Vec::new();
        let mut inflections = Vec::new();
        let mut pos = None;
        let mut domain = None;
        for (mark, text) in tokenize(&strip_html(&body)) {
            match mark {
                '@' => domain = Some(text),
                '*' => {
                    pos = canonical_pos(&text);
                    inflections_from_pos(&headword, &text, &mut inflections);
                }
                '-' | '!' | '\0' => {
                    if !text.is_empty() {
                        senses.push(SenseRec {
                            pos: pos.clone(),
                            gloss: text,
                            domain: domain.clone(),
                            examples: Vec::new(),
                        });
                    }
                }
                '=' => {
                    if let Some(sense) = senses.last_mut() {
                        let (src, dst) = text
                            .split_once('+')
                            .map_or((text.as_str(), None), |(a, b)| (a, Some(b)));
                        sense.examples.push(ExampleRec {
                            src: src.trim().into(),
                            dst: dst
                                .map(str::trim)
                                .filter(|x| !x.is_empty())
                                .map(str::to_string),
                        });
                    }
                }
                _ => {}
            }
        }
        if senses.is_empty() {
            stats.skipped += 1;
            continue;
        }
        stats.records += 1;
        records.push(Record {
            headword,
            lang: lang.into(),
            senses,
            inflections,
            source: "hongocduc".into(),
            ..Default::default()
        });
    }
    Ok((records, stats))
}
