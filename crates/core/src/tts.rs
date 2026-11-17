//! Trừu tượng phát âm. Cài đặt cụ thể (`OsSpeaker`, `PiperSpeaker`) nằm ngoài `core`
//! để thư viện lõi không phụ thuộc API âm thanh của hệ điều hành.

/// Giọng đọc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Accent {
    /// Anh – Mỹ.
    #[default]
    Us,
    /// Anh – Anh.
    Uk,
    /// Tiếng Việt.
    Vi,
}

impl Accent {
    /// Mã BCP-47 tương ứng.
    pub fn bcp47(self) -> &'static str {
        match self {
            Accent::Us => "en-US",
            Accent::Uk => "en-GB",
            Accent::Vi => "vi-VN",
        }
    }
}

/// Lỗi phát âm.
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    /// Hệ điều hành không có TTS / không khởi tạo được.
    #[error("không khởi tạo được bộ đọc: {0}")]
    Init(String),
    /// Lỗi khi đọc.
    #[error("lỗi phát âm: {0}")]
    Speak(String),
}

/// Bộ phát âm. Phải gọi được từ thread nền.
pub trait Speaker: Send + Sync {
    /// Đọc `text` với giọng `accent`. Không được chặn lâu (nên đọc bất đồng bộ).
    fn speak(&self, text: &str, accent: Accent) -> Result<(), TtsError>;
}

/// Bộ phát âm rỗng (khi hệ điều hành không hỗ trợ).
#[derive(Debug, Default)]
pub struct NullSpeaker;

impl Speaker for NullSpeaker {
    fn speak(&self, _text: &str, _accent: Accent) -> Result<(), TtsError> {
        Err(TtsError::Init("không có bộ đọc".into()))
    }
}
