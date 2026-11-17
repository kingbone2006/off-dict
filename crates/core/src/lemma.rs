//! Lemmatizer tiếng Anh: bảng bất quy tắc (nhúng) + quy tắc hậu tố.
//!
//! Hàm chỉ sinh **ứng viên**; tầng `search` sẽ kiểm tra ứng viên nào có trong từ điển.

use std::collections::HashMap;
use std::sync::OnceLock;

const VERBS_TSV: &str = include_str!("../../../data/lists/irregular_verbs.tsv");
const PLURALS_TSV: &str = include_str!("../../../data/lists/irregular_plurals.tsv");

/// Bảng ánh xạ dạng biến thể → lemma (bất quy tắc).
fn irregular_table() -> &'static HashMap<&'static str, Vec<&'static str>> {
    static TABLE: OnceLock<HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
        let mut add = |form: &'static str, lemma: &'static str| {
            if form != lemma {
                let v = map.entry(form).or_default();
                if !v.contains(&lemma) {
                    v.push(lemma);
                }
            }
        };
        for line in VERBS_TSV.lines().filter(|l| !l.starts_with('#')) {
            let mut cols = line.split('\t');
            let Some(base) = cols.next().map(str::trim).filter(|s| !s.is_empty()) else {
                continue;
            };
            for col in cols {
                for form in col.split('/') {
                    add(form.trim(), base);
                }
            }
        }
        for line in PLURALS_TSV.lines().filter(|l| !l.starts_with('#')) {
            let mut cols = line.split('\t');
            if let (Some(sing), Some(plur)) = (cols.next(), cols.next()) {
                add(plur.trim(), sing.trim());
            }
        }
        map
    })
}

/// Tất cả cặp (dạng biến thể, lemma) bất quy tắc — dùng cho `dict-builder`.
pub fn irregular_pairs() -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<_> = irregular_table()
        .iter()
        .flat_map(|(f, ls)| ls.iter().map(move |l| (*f, *l)))
        .collect();
    v.sort_unstable();
    v
}

/// Lemma bất quy tắc của `form` (đã chuẩn hoá chữ thường).
pub fn irregular(form: &str) -> &'static [&'static str] {
    irregular_table()
        .get(form)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Sinh ứng viên lemma theo bảng bất quy tắc + quy tắc hậu tố, theo thứ tự ưu tiên.
///
/// ```
/// let c = vietdict_core::lemma::candidates("running");
/// assert!(c.contains(&"run".to_string()));
/// ```
pub fn candidates(word: &str) -> Vec<String> {
    let mut out: Vec<String> = irregular(word).iter().map(|s| s.to_string()).collect();
    if !word.is_ascii() || word.len() < 3 || word.contains(' ') {
        return out;
    }
    let mut push = |s: String| {
        if s.len() >= 2 && s != word && !out.contains(&s) {
            out.push(s);
        }
    };
    let strip = |suf: &str| word.strip_suffix(suf).map(str::to_string);

    if let Some(stem) = strip("ies") {
        push(format!("{stem}y"));
    }
    if let Some(stem) = strip("ied") {
        push(format!("{stem}y"));
    }
    if let Some(stem) = strip("ves") {
        push(format!("{stem}f"));
        push(format!("{stem}fe"));
    }
    for suf in ["sses", "shes", "ches", "xes", "zes", "oes"] {
        if word.ends_with(suf) {
            if let Some(stem) = strip("es") {
                push(stem);
            }
        }
    }
    if let Some(stem) = strip("ing") {
        push_verb_stem(&stem, &mut push);
    }
    if let Some(stem) = strip("ed") {
        push_verb_stem(&stem, &mut push);
    }
    if let Some(stem) = strip("est") {
        push_adj_stem(&stem, &mut push);
    }
    if let Some(stem) = strip("er") {
        push_adj_stem(&stem, &mut push);
    }
    if let Some(stem) = strip("ly") {
        if let Some(s) = stem.strip_suffix('i') {
            push(format!("{s}y"));
        }
        push(stem);
    }
    if !word.ends_with("ss") && !word.ends_with("us") && !word.ends_with("is") {
        if let Some(stem) = strip("s") {
            push(stem);
        }
    }
    if let Some(stem) = strip("'s") {
        push(stem);
    }
    out
}

fn push_verb_stem(stem: &str, push: &mut impl FnMut(String)) {
    if stem.len() < 2 {
        return;
    }
    let b = stem.as_bytes();
    let n = b.len();
    // Phụ âm đôi: running → run, stopped → stop.
    if n >= 3 && b[n - 1] == b[n - 2] && !matches!(b[n - 1], b'l' | b's' | b'z' | b'f') {
        push(stem[..n - 1].to_string());
    }
    push(stem.to_string());
    // making → make, hoped → hope.
    push(format!("{stem}e"));
    // dying → die.
    if let Some(s) = stem.strip_suffix('y') {
        push(format!("{s}ie"));
    }
}

fn push_adj_stem(stem: &str, push: &mut impl FnMut(String)) {
    if stem.len() < 2 {
        return;
    }
    let b = stem.as_bytes();
    let n = b.len();
    if let Some(s) = stem.strip_suffix('i') {
        push(format!("{s}y"));
    }
    if n >= 3 && b[n - 1] == b[n - 2] {
        push(stem[..n - 1].to_string());
    }
    push(stem.to_string());
    push(format!("{stem}e"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(word: &str, lemma: &str) -> bool {
        candidates(word).iter().any(|c| c == lemma)
    }

    #[test]
    fn irregulars() {
        assert_eq!(irregular("went"), &["go"]);
        assert_eq!(irregular("children"), &["child"]);
        assert_eq!(irregular("mice"), &["mouse"]);
        assert!(irregular("ran").contains(&"run"));
    }

    #[test]
    fn suffixes() {
        assert!(has("running", "run"));
        assert!(has("making", "make"));
        assert!(has("stopped", "stop"));
        assert!(has("cities", "city"));
        assert!(has("boxes", "box"));
        assert!(has("cats", "cat"));
        assert!(has("happier", "happy"));
        assert!(has("biggest", "big"));
        assert!(has("dying", "die"));
        assert!(has("knives", "knife"));
    }

    #[test]
    fn pairs_not_empty() {
        assert!(irregular_pairs().len() > 200);
    }
}
