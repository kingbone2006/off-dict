//! Xếp hạng gợi ý (mục 9.3 của AGENTS.md).

/// Các tín hiệu dùng để tính điểm.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Signals {
    /// Khớp chính xác có dấu.
    pub exact: bool,
    /// Khớp chính xác sau khi bỏ dấu.
    pub exact_no_diacritics: bool,
    /// Khớp qua lemma.
    pub lemma: bool,
    /// Lemma lấy từ bảng biến thể (bất quy tắc / "xem X") — tín hiệu mạnh hơn quy tắc hậu tố.
    pub strong_lemma: bool,
    /// Khớp tiền tố.
    pub prefix: bool,
    /// Khoảng cách chỉnh sửa (0 nếu không fuzzy).
    pub edit_distance: u32,
    /// Độ phổ biến.
    pub frequency: i64,
    /// Có trong lịch sử người dùng.
    pub in_history: bool,
    /// Tiền tố khớp cả dấu (truy vấn có dấu).
    pub prefix_with_diacritics: bool,
    /// Độ dài chênh lệch giữa key và truy vấn (ký tự) — phạt nhẹ cho prefix dài.
    pub extra_len: u32,
}

/// Trọng số của tần suất. Tài liệu gốc dùng 10; vì `frequency` là số lần xuất hiện
/// thô trong corpus (tới ~10^7), hệ số 4 giữ tần suất không lấn át khớp chính xác.
/// Xem ADR-005 trong `docs/DECISIONS.md`.
pub const FREQ_WEIGHT: f64 = 4.0;

/// Tính điểm theo công thức trong tài liệu (mục 9.3), có tinh chỉnh nhẹ.
pub fn score(s: &Signals) -> f64 {
    let b = |x: bool| if x { 1.0 } else { 0.0 };
    100.0 * b(s.exact)
        + 60.0 * b(s.exact_no_diacritics)
        + 50.0 * b(s.lemma)
        + 125.0 * b(s.strong_lemma)
        + 30.0 * b(s.prefix)
        + 20.0 * b(s.prefix_with_diacritics)
        - 15.0 * f64::from(s.edit_distance)
        + FREQ_WEIGHT * (1.0 + s.frequency.max(0) as f64).ln()
        + 5.0 * b(s.in_history)
        - 0.5 * f64::from(s.extra_len.min(40))
}

/// Khoảng cách chỉnh sửa OSA (Levenshtein + hoán vị 2 ký tự kề nhau), theo ký tự.
pub fn levenshtein(a: &str, b: &str) -> u32 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0u32; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i as u32;
    }
    for j in 0..=m {
        d[0][j] = j as u32;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = u32::from(a[i - 1] != b[j - 1]);
            let mut v = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = v;
        }
    }
    d[n][m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_beats_prefix() {
        let e = score(&Signals {
            exact: true,
            ..Default::default()
        });
        let p = score(&Signals {
            prefix: true,
            frequency: 1000,
            ..Default::default()
        });
        assert!(e > p);
    }

    #[test]
    fn lev() {
        assert_eq!(levenshtein("recieve", "receive"), 1);
        assert_eq!(levenshtein("recieve", "relieve"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("cat", "cat"), 0);
        assert_eq!(levenshtein("mèo", "meo"), 1);
    }
}
