//! Tìm kiếm: exact → không dấu → telex → lemma → prefix → fuzzy; tra ngược FTS5.

use std::collections::{HashMap, HashSet};

use fst::automaton::{Levenshtein, Str};
use fst::{Automaton, IntoStreamer, Streamer};
use rusqlite::Connection;

use crate::dict_repo::{Dictionary, EntryHead};
use crate::normalize::{has_diacritics, normalize, strip_diacritics, truncate_query};
use crate::rank::{levenshtein, score, Signals};
use crate::schema::unpack_value;
use crate::{lemma, telex, DictError, Lang, MatchKind, Suggestion};

/// Số key tối đa quét trong FST cho một truy vấn prefix.
const PREFIX_SCAN: usize = 20_000;
/// Số key tối đa lấy từ automaton fuzzy.
const FUZZY_SCAN: usize = 2_000;

#[derive(Debug, Clone)]
pub(crate) struct Cand {
    pub id: i64,
    pub headword: String,
    pub score: f64,
    pub kind: MatchKind,
}

fn kind_of(s: &Signals, telex_hit: bool) -> MatchKind {
    if telex_hit {
        MatchKind::Telex
    } else if s.lemma {
        MatchKind::Lemma
    } else if s.exact {
        MatchKind::Exact
    } else if s.exact_no_diacritics {
        MatchKind::NoDiacritics
    } else if s.prefix {
        MatchKind::Prefix
    } else {
        MatchKind::Fuzzy
    }
}

struct Collector<'h> {
    map: HashMap<i64, Cand>,
    history: &'h HashSet<String>,
}

impl Collector<'_> {
    fn add(&mut self, head: &EntryHead, mut sig: Signals, telex_hit: bool) {
        sig.frequency = head.frequency;
        sig.in_history = self.history.contains(&normalize(&head.headword));
        let sc = score(&sig);
        if self.map.get(&head.id).is_some_and(|old| old.score >= sc) {
            return;
        }
        let cand = Cand {
            id: head.id,
            headword: head.headword.clone(),
            score: sc,
            kind: kind_of(&sig, telex_hit),
        };
        self.map.insert(head.id, cand);
    }
}

pub(crate) fn finish(
    conn: &Connection,
    mut cands: Vec<Cand>,
    lang: Lang,
    limit: usize,
) -> Result<Vec<Suggestion>, DictError> {
    cands.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.headword.cmp(&b.headword))
    });
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(limit);
    for c in cands {
        if !seen.insert(normalize(&c.headword)) {
            continue;
        }
        let preview = Dictionary::preview(conn, c.id)?;
        out.push(Suggestion {
            entry_id: c.id,
            headword: c.headword,
            lang,
            preview,
            score: c.score,
            kind: c.kind,
        });
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

impl Dictionary {
    /// Gợi ý khi đang gõ (F03–F06). Không bao giờ lỗi: lỗi nội bộ → danh sách rỗng.
    pub fn suggest(&self, query: &str, lang: Lang, limit: usize) -> Vec<Suggestion> {
        self.suggest_with_history(query, lang, limit, &HashSet::new())
            .unwrap_or_default()
    }

    /// Như [`Dictionary::suggest`] nhưng cộng điểm cho từ có trong `history`
    /// (tập `normalize(headword)`) và trả lỗi nếu có.
    pub fn suggest_with_history(
        &self,
        query: &str,
        lang: Lang,
        limit: usize,
        history: &HashSet<String>,
    ) -> Result<Vec<Suggestion>, DictError> {
        let q = normalize(truncate_query(query));
        if q.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let q_nd = strip_diacritics(&q);
        let conn = self.conn()?;
        let mut col = Collector {
            map: HashMap::new(),
            history,
        };

        // 1–2. Khớp chính xác (có dấu / không dấu).
        for h in Self::heads_by_norm(&conn, lang, &q_nd)? {
            let exact = normalize(&h.headword) == q;
            col.add(
                &h,
                Signals {
                    exact,
                    exact_no_diacritics: true,
                    ..Default::default()
                },
                false,
            );
        }
        // 2b. Telex thô (tiếng Việt): meof → mèo.
        if lang == Lang::Vi && !has_diacritics(&q) {
            if let Some(t) = telex::telex_to_vi(&q) {
                for h in Self::heads_by_norm(&conn, lang, &strip_diacritics(&t))? {
                    if normalize(&h.headword) == t {
                        col.add(
                            &h,
                            Signals {
                                exact: true,
                                ..Default::default()
                            },
                            true,
                        );
                    }
                }
            }
        }
        // 3. Lemma (tiếng Anh).
        if lang == Lang::En {
            Self::collect_lemmas(&conn, &q, &mut col)?;
        }
        // 4. Prefix.
        let pre_count = self.collect_prefix(&conn, lang, &q, &q_nd, limit, &mut col)?;
        // 5. Fuzzy — chỉ khi các bước trên cho ít hơn `limit` kết quả.
        if col.map.len() + pre_count < limit {
            self.collect_fuzzy(&conn, lang, &q_nd, limit, &mut col)?;
        }
        finish(&conn, col.map.into_values().collect(), lang, limit)
    }

    fn collect_lemmas(
        conn: &Connection,
        q: &str,
        col: &mut Collector<'_>,
    ) -> Result<(), DictError> {
        // Bảng inflection trong dữ liệu + bảng bất quy tắc nhúng → tín hiệu mạnh.
        let mut strong: Vec<String> = Self::inflection_lemmas(conn, q)?
            .into_iter()
            .map(|(l, _)| normalize(&l))
            .filter(|l| l != q)
            .collect();
        for l in lemma::irregular(q) {
            if !strong.iter().any(|s| s == l) {
                strong.push((*l).to_string());
            }
        }
        let weak: Vec<String> = lemma::candidates(q)
            .into_iter()
            .filter(|c| !strong.contains(c))
            .collect();
        for (list, is_strong) in [(strong, true), (weak, false)] {
            for l in list {
                for h in Self::heads_by_norm(conn, Lang::En, &strip_diacritics(&l))? {
                    if normalize(&h.headword) == l {
                        let sig = Signals {
                            lemma: true,
                            strong_lemma: is_strong,
                            ..Default::default()
                        };
                        col.add(&h, sig, false);
                    }
                }
            }
        }
        Ok(())
    }

    fn collect_prefix(
        &self,
        conn: &Connection,
        lang: Lang,
        q: &str,
        q_nd: &str,
        limit: usize,
        col: &mut Collector<'_>,
    ) -> Result<usize, DictError> {
        let q_len = q_nd.chars().count() as u32;
        let mut pre: Vec<(String, i64, i64)> = Vec::new();
        let mut stream = self
            .fst(lang)
            .search(Str::new(q_nd).starts_with())
            .into_stream();
        while let Some((k, v)) = stream.next() {
            if k != q_nd.as_bytes() {
                let (id, freq) = unpack_value(v);
                pre.push((String::from_utf8_lossy(k).into_owned(), id, freq));
                if pre.len() >= PREFIX_SCAN {
                    break;
                }
            }
        }
        let count = pre.len();
        let sig = |key: &str, freq: i64| Signals {
            prefix: true,
            frequency: freq,
            extra_len: (key.chars().count() as u32).saturating_sub(q_len),
            ..Default::default()
        };
        pre.sort_by(|a, b| score(&sig(&b.0, b.2)).total_cmp(&score(&sig(&a.0, a.2))));
        let marks = has_diacritics(q);
        for (key, id, _) in pre.into_iter().take(limit * 3) {
            if let Some(h) = Self::head_by_id(conn, id)? {
                let mut s = sig(&key, h.frequency);
                s.prefix_with_diacritics = marks && normalize(&h.headword).starts_with(q);
                col.add(&h, s, false);
            }
        }
        Ok(count)
    }

    fn collect_fuzzy(
        &self,
        conn: &Connection,
        lang: Lang,
        q_nd: &str,
        limit: usize,
        col: &mut Collector<'_>,
    ) -> Result<(), DictError> {
        let q_len = q_nd.chars().count();
        if !(2..=32).contains(&q_len) {
            return Ok(());
        }
        let dist = if q_len <= 5 { 1 } else { 2 };
        let Ok(aut) = Levenshtein::new(q_nd, dist) else {
            return Ok(());
        };
        let mut fz: Vec<(i64, i64, u32)> = Vec::new();
        let mut stream = self.fst(lang).search(&aut).into_stream();
        while let Some((k, v)) = stream.next() {
            let (id, freq) = unpack_value(v);
            fz.push((id, freq, levenshtein(q_nd, &String::from_utf8_lossy(k))));
            if fz.len() >= FUZZY_SCAN {
                break;
            }
        }
        let sig = |freq: i64, d: u32| Signals {
            edit_distance: d,
            frequency: freq,
            ..Default::default()
        };
        fz.sort_by(|a, b| score(&sig(b.1, b.2)).total_cmp(&score(&sig(a.1, a.2))));
        for (id, _, d) in fz.into_iter().take(limit * 3) {
            if let Some(h) = Self::head_by_id(conn, id)? {
                col.add(&h, sig(h.frequency, d), false);
            }
        }
        Ok(())
    }
}

/// Tạo truy vấn FTS5 an toàn: mỗi token đặt trong ngoặc kép, nối bằng khoảng trắng (AND),
/// toàn bộ cụm cũng được thử như một phrase để ưu tiên khớp liền.
fn fts_query(text: &str) -> Option<String> {
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .take(8)
        .map(|t| format!("\"{t}\""))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

impl Dictionary {
    /// Tra ngược trong phần nghĩa (F02 / mục 9.2 bước 6): `"con mèo"` → `cat`.
    ///
    /// `lang` là ngôn ngữ của **mục từ** muốn tìm (vd. `Lang::En` để tìm từ tiếng Anh có
    /// nghĩa chứa "con mèo").
    pub fn reverse_lookup(
        &self,
        text: &str,
        lang: Lang,
        limit: usize,
    ) -> Result<Vec<Suggestion>, DictError> {
        let q = normalize(truncate_query(text));
        let Some(fts) = fts_query(&q) else {
            return Ok(Vec::new());
        };
        let conn = self.conn()?;
        let mut st = conn.prepare_cached(
            "SELECT e.id, e.headword, e.frequency, s.gloss, s.ord, f.rank
             FROM sense_fts f
             JOIN sense s ON s.id = f.rowid
             JOIN entry e ON e.id = s.entry_id
             WHERE sense_fts MATCH ?1 AND e.lang = ?2
             ORDER BY f.rank LIMIT 400",
        )?;
        type Row = (i64, String, i64, String, i64, f64);
        let rows: Vec<Row> = st
            .query_map(rusqlite::params![fts, lang.as_str()], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?
            .collect::<Result<_, _>>()?;
        let q_nd = strip_diacritics(&q);
        let mut best: HashMap<i64, Cand> = HashMap::new();
        for (id, headword, freq, gloss, ord, rank) in rows {
            let g = normalize(&gloss);
            // Nghĩa đúng bằng truy vấn (sau khi bỏ chú thích ngoặc) là tín hiệu mạnh nhất.
            let core_gloss: String = g
                .split(['(', ',', ';'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            let exact = core_gloss == q || strip_diacritics(&core_gloss) == q_nd;
            let sc = 60.0 * f64::from(u8::from(exact)) + 25.0 * f64::from(u8::from(g.contains(&q)))
                - 4.0 * (ord.min(10) as f64)
                - 0.5 * rank.clamp(-20.0, 0.0)
                + crate::rank::FREQ_WEIGHT * (1.0 + freq.max(0) as f64).ln()
                - 0.3 * (headword.chars().count().min(30) as f64);
            if best.get(&id).is_some_and(|c| c.score >= sc) {
                continue;
            }
            best.insert(
                id,
                Cand {
                    id,
                    headword,
                    score: sc,
                    kind: MatchKind::Reverse,
                },
            );
        }
        finish(&conn, best.into_values().collect(), lang, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::fts_query;

    #[test]
    fn fts_escape() {
        assert_eq!(fts_query("con mèo").as_deref(), Some("\"con\" \"mèo\""));
        assert_eq!(fts_query("\"; DROP"), Some("\"DROP\"".to_string()));
        assert_eq!(fts_query("  ...  "), None);
    }
}
