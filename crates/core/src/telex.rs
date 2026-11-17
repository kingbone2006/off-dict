//! Chuyển Telex thô → tiếng Việt có dấu (`meof` → `mèo`, `duowngf` → `đường`).
//!
//! Chỉ dùng như một nhánh truy vấn bổ sung, không thay thế bộ gõ.

use unicode_normalization::UnicodeNormalization;

const VOWELS: &str = "aăâeêioôơuưy";

fn is_vowel(c: char) -> bool {
    VOWELS.contains(c)
}

fn tone_mark(c: char) -> Option<char> {
    match c {
        's' => Some('\u{0301}'),
        'f' => Some('\u{0300}'),
        'r' => Some('\u{0309}'),
        'x' => Some('\u{0303}'),
        'j' => Some('\u{0323}'),
        _ => None,
    }
}

/// Chuyển một chuỗi gõ kiểu Telex sang tiếng Việt có dấu.
///
/// Trả về `None` nếu không có phép biến đổi Telex nào được áp dụng.
///
/// ```
/// use vietdict_core::telex::telex_to_vi;
/// assert_eq!(telex_to_vi("meof").as_deref(), Some("mèo"));
/// assert_eq!(telex_to_vi("dduowngf").as_deref(), Some("đường"));
/// assert_eq!(telex_to_vi("duowngf").as_deref(), Some("dường"));
/// assert_eq!(telex_to_vi("con meo"), None);
/// ```
pub fn telex_to_vi(s: &str) -> Option<String> {
    let input = crate::normalize::normalize(s);
    if input.is_empty() || !input.is_ascii() {
        return None;
    }
    let words: Vec<String> = input.split(' ').map(convert_word).collect();
    let out = words.join(" ");
    if out == input {
        None
    } else {
        Some(out)
    }
}

fn convert_word(word: &str) -> String {
    let mut out: Vec<char> = Vec::with_capacity(word.len());
    let mut tone: Option<char> = None;
    for c in word.chars() {
        let has_vowel = out.iter().any(|&v| is_vowel(v));
        if has_vowel {
            if let Some(mark) = tone_mark(c) {
                tone = Some(mark);
                continue;
            }
            if c == 'z' {
                tone = None;
                continue;
            }
        }
        match c {
            'w' => apply_w(&mut out),
            'a' | 'e' | 'o' if out.last() == Some(&c) => {
                if let Some(last) = out.last_mut() {
                    *last = match c {
                        'a' => 'â',
                        'e' => 'ê',
                        _ => 'ô',
                    };
                }
            }
            'd' if out.last() == Some(&'d') => {
                if let Some(last) = out.last_mut() {
                    *last = 'đ';
                }
            }
            _ => out.push(c),
        }
    }
    if let Some(mark) = tone {
        if let Some(pos) = tone_position(&out) {
            let mut res = String::with_capacity(out.len() * 2);
            for (i, &ch) in out.iter().enumerate() {
                res.push(ch);
                if i == pos {
                    res.push(mark);
                }
            }
            return res.nfc().collect();
        }
    }
    out.into_iter().collect()
}

fn apply_w(out: &mut Vec<char>) {
    let last_vowel = out.iter().rposition(|&c| is_vowel(c));
    match last_vowel {
        Some(i) => {
            let c = out[i];
            if c == 'o' && i > 0 && out[i - 1] == 'u' {
                out[i - 1] = 'ư';
                out[i] = 'ơ';
            } else {
                match c {
                    'a' => out[i] = 'ă',
                    'o' => out[i] = 'ơ',
                    'u' => out[i] = 'ư',
                    _ => out.push('w'),
                }
            }
        }
        None => out.push('ư'),
    }
}

/// Chọn vị trí đặt dấu thanh theo quy tắc chính tả (kiểu mới, giản lược).
fn tone_position(chars: &[char]) -> Option<usize> {
    let mut start = 0;
    // "qu" và "gi" (khi theo sau là nguyên âm): u/i là phụ âm.
    if chars.len() > 2 && chars[0] == 'q' && chars[1] == 'u' {
        start = 2;
    } else if chars.len() > 2 && chars[0] == 'g' && chars[1] == 'i' && is_vowel(chars[2]) {
        start = 2;
    }
    let vowels: Vec<usize> = (start..chars.len())
        .filter(|&i| is_vowel(chars[i]))
        .collect();
    let (&first, &last) = (vowels.first()?, vowels.last()?);
    if let Some(&i) = vowels.iter().rev().find(|&&i| "ăâêôơư".contains(chars[i])) {
        return Some(i);
    }
    if vowels.len() == 1 {
        return Some(first);
    }
    let has_final = last + 1 < chars.len();
    if has_final {
        return Some(last);
    }
    if vowels.len() == 2 {
        let pair: String = [chars[first], chars[last]].iter().collect();
        if matches!(pair.as_str(), "oa" | "oe" | "uy") {
            return Some(last);
        }
        return Some(first);
    }
    Some(vowels[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases() {
        let cases = [
            ("meof", "mèo"),
            ("dduowngf", "đường"),
            ("duowngf", "dường"),
            ("tieengs", "tiếng"),
            ("vieetj", "việt"),
            ("hoaf", "hoà"),
            ("nguwowif", "người"),
            ("quas", "quá"),
            ("gias", "giá"),
            ("ddaay", "đây"),
            ("con meof", "con mèo"),
        ];
        for (i, e) in cases {
            assert_eq!(telex_to_vi(i).as_deref(), Some(e), "input {i}");
        }
    }

    #[test]
    fn no_change() {
        assert_eq!(telex_to_vi("con"), None);
        assert_eq!(telex_to_vi(""), None);
        assert_eq!(telex_to_vi("mèo"), None);
    }
}
