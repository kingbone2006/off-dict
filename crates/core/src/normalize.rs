//! Chuẩn hoá chuỗi — dùng chung cho `dict-builder` và app.

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Độ dài tối đa (ký tự) của truy vấn dùng cho gợi ý.
pub const MAX_QUERY_CHARS: usize = 64;

/// NFC + chữ thường + cắt/gộp khoảng trắng.
///
/// ```
/// assert_eq!(vietdict_core::normalize::normalize("  Con   MÈO "), "con mèo");
/// ```
pub fn normalize(s: &str) -> String {
    let lowered: String = s.nfc().flat_map(char::to_lowercase).collect();
    // Lowercase có thể sinh chuỗi không NFC (hiếm) → NFC lại lần nữa.
    let nfc: String = lowered.nfc().collect();
    collapse_ws(&nfc)
}

/// Bỏ dấu tiếng Việt (và mọi dấu kết hợp), đổi `đ → d`, đồng thời chuẩn hoá.
///
/// ```
/// assert_eq!(vietdict_core::normalize::strip_diacritics("Đường"), "duong");
/// ```
pub fn strip_diacritics(s: &str) -> String {
    let base = normalize(s);
    let stripped: String = base
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| match c {
            'đ' | 'Đ' => 'd',
            other => other,
        })
        .collect();
    stripped.nfc().collect()
}

/// Cắt truy vấn còn tối đa [`MAX_QUERY_CHARS`] ký tự.
pub fn truncate_query(s: &str) -> &str {
    match s.char_indices().nth(MAX_QUERY_CHARS) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// `true` nếu chuỗi có ký tự ngoài ASCII (thường là có dấu).
pub fn has_diacritics(s: &str) -> bool {
    !s.is_ascii()
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(normalize("  Hello   World "), "hello world");
        assert_eq!(strip_diacritics("nghiêng"), "nghieng");
        assert_eq!(strip_diacritics("ĐƯỜNG"), "duong");
    }

    #[test]
    fn nfd_equals_nfc() {
        let nfd: String = "mèo".nfd().collect();
        assert_eq!(normalize(&nfd), normalize("mèo"));
    }

    #[test]
    fn truncate() {
        let long = "a".repeat(200);
        assert_eq!(truncate_query(&long).len(), MAX_QUERY_CHARS);
        assert_eq!(truncate_query("ờ"), "ờ");
    }
}
