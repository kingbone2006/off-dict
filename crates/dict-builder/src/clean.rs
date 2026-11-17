//! Làm sạch văn bản: bỏ HTML, sửa lỗi mã hoá (mojibake), thống nhất từ loại.

use vietdict_core::normalize::normalize;

/// Bỏ thẻ HTML và giải mã vài thực thể phổ biến.
pub fn strip_html(s: &str) -> String {
    if !s.contains(['<', '&']) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            // Chỉ coi là thẻ khi theo sau là chữ cái hoặc '/' (tránh "a < b").
            '<' if chars
                .get(i + 1)
                .is_some_and(|n| n.is_ascii_alphabetic() || *n == '/') =>
            {
                in_tag = true
            }
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Đếm dấu hiệu mojibake (UTF-8 bị đọc thành Latin-1/CP1252).
fn mojibake_score(s: &str) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let pairs = chars
        .windows(2)
        .filter(|w| {
            matches!(w[0], 'Ã' | 'Â' | 'Æ' | 'Ä' | 'á' | 'â')
                && (0x80..=0xBF).contains(&u32::from(w[1]))
        })
        .count();
    pairs + chars.iter().filter(|&&c| c == '\u{fffd}').count()
}

/// Thử sửa mojibake: mã hoá lại thành Latin-1 rồi giải mã UTF-8.
/// Trả về `Some(sửa)` nếu chuỗi sửa hợp lệ và "sạch" hơn.
pub fn fix_mojibake(s: &str) -> Option<String> {
    if mojibake_score(s) == 0 {
        return None;
    }
    let bytes: Option<Vec<u8>> = s.chars().map(|c| u8::try_from(u32::from(c)).ok()).collect();
    let fixed = String::from_utf8(bytes?).ok()?;
    (mojibake_score(&fixed) < mojibake_score(s)).then_some(fixed)
}

/// Có ký tự lỗi mã hoá không (dùng cho `validate`).
///
/// Chữ `Ã` hợp lệ trong tiếng Việt viết hoa (`HÃNG`), nên chỉ coi là lỗi khi đi kèm
/// ký tự Latin-1 đặc trưng của mojibake (U+0080..U+00BF) hoặc có `�`.
pub fn has_bad_chars(s: &str) -> bool {
    mojibake_score(s) > 0
}

/// Sửa rác thường gặp trong dữ liệu gốc: `Ê-Ãcốt` → `Ê-cốt`, `(A-ÃRập)` → `(A-Rập)`.
pub fn fix_stray_capital_a_tilde(s: &str) -> Option<String> {
    if !s.contains('Ã') {
        return None;
    }
    let chars: Vec<char> = s.chars().collect();
    let mut changed = false;
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + 1).copied();
        let next2 = chars.get(i + 2).copied();
        let after_sep = matches!(prev, Some('-' | ' ' | '('));
        let lower_next = next.is_some_and(char::is_lowercase);
        let cap_then_lower =
            next.is_some_and(char::is_uppercase) && next2.is_some_and(char::is_lowercase);
        if c == 'Ã' && after_sep && (lower_next || cap_then_lower) {
            changed = true;
            continue;
        }
        out.push(c);
    }
    changed.then_some(out)
}

const POS_MAP: &[(&[&str], &str)] = &[
    (
        &[
            "ngoại động từ",
            "nội động từ",
            "động từ",
            "trợ động từ",
            "verb",
            "v.",
            "v",
            "vt",
            "vi",
            "transitive verb",
            "intransitive verb",
        ],
        "verb",
    ),
    (&["danh từ", "noun", "n.", "n"], "noun"),
    (&["tính từ", "adj", "adj.", "adjective", "a."], "adj"),
    (&["phó từ", "trạng từ", "adv", "adv.", "adverb"], "adv"),
    (&["giới từ", "prep", "prep.", "preposition"], "prep"),
    (&["liên từ", "conj", "conj.", "conjunction"], "conj"),
    (&["đại từ", "pronoun", "pron", "pron."], "pron"),
    (
        &["thán từ", "interj", "interj.", "exclamation", "excl"],
        "interj",
    ),
    (&["mạo từ", "article", "art"], "article"),
    (&["từ hạn định", "determiner", "det"], "det"),
    (&["số từ", "numeral", "num"], "num"),
    (&["tiền tố", "prefix"], "prefix"),
    (&["hậu tố", "suffix"], "suffix"),
    (&["viết tắt", "abbr", "abbr.", "abbreviation"], "abbr"),
];

/// Thống nhất nhãn từ loại → `noun`, `verb`, `adj`, ...
pub fn canonical_pos(raw: &str) -> Option<String> {
    let s = normalize(raw.trim_matches(|c: char| c.is_whitespace() || c == ':' || c == ','));
    if s.is_empty() {
        return None;
    }
    // "danh từ, số nhiều children" → lấy phần trước dấu phẩy.
    let head = s.split(',').next().unwrap_or(&s).trim();
    for (keys, canon) in POS_MAP {
        if keys.contains(&head) {
            return Some((*canon).to_string());
        }
    }
    for (keys, canon) in POS_MAP {
        if keys
            .iter()
            .filter(|k| k.chars().count() > 3)
            .any(|k| head.starts_with(*k))
        {
            return Some((*canon).to_string());
        }
    }
    // "verb & adj" → lấy nhãn đầu.
    if let Some(first) = head.split(['&', '/']).next() {
        if first.trim() != head {
            return canonical_pos(first);
        }
    }
    Some(head.to_string())
}

/// Gộp khoảng trắng, cắt hai đầu và bỏ ký tự điều khiển.
pub fn tidy(s: &str) -> String {
    let no_ctrl: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    no_ctrl.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html() {
        assert_eq!(
            strip_html("<b>con</b> m\u{e8}o &amp; a < b"),
            "con m\u{e8}o & a < b"
        );
    }

    #[test]
    fn mojibake() {
        assert_eq!(fix_mojibake("m\u{c3}\u{a8}o").as_deref(), Some("m\u{e8}o"));
        assert_eq!(fix_mojibake("m\u{e8}o"), None);
        assert!(has_bad_chars("m\u{c3}\u{a8}o"));
        assert!(has_bad_chars("x\u{fffd}"));
        assert!(!has_bad_chars("L\u{c3}I SU\u{1ea4}T V\u{c0} H\u{c3}NG"));
    }

    #[test]
    fn stray() {
        assert_eq!(
            fix_stray_capital_a_tilde("\u{ca}-\u{c3}c\u{1ed1}t").as_deref(),
            Some("\u{ca}-c\u{1ed1}t")
        );
        assert_eq!(
            fix_stray_capital_a_tilde("(A-\u{c3}R\u{1ead}p)").as_deref(),
            Some("(A-R\u{1ead}p)")
        );
        assert_eq!(fix_stray_capital_a_tilde("H\u{c3}NG"), None);
    }

    #[test]
    fn pos() {
        assert_eq!(
            canonical_pos("ngo\u{1ea1}i \u{111}\u{1ed9}ng t\u{1eeb}").as_deref(),
            Some("verb")
        );
        assert_eq!(
            canonical_pos("danh t\u{1eeb},  s\u{1ed1} nhi\u{1ec1}u children").as_deref(),
            Some("noun")
        );
        assert_eq!(canonical_pos("noun").as_deref(), Some("noun"));
        assert_eq!(canonical_pos("verb & adj").as_deref(), Some("verb"));
        assert_eq!(canonical_pos("n.").as_deref(), Some("noun"));
    }
}
