use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::SharedString;
use vietdict_core::{Dictionary, Lang};

slint::include_modules!();

fn data_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VIETDICT_DATA") {
        return Some(PathBuf::from(path));
    }
    let exe = std::env::current_exe().ok()?;
    let beside = exe.parent()?.join("data");
    if beside.is_dir() {
        return Some(beside);
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/build");
    dev.is_dir().then_some(dev)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    let dictionary = data_dir()
        .and_then(|dir| Dictionary::open(&dir).ok())
        .map(Rc::new);
    let lang = Rc::new(std::cell::Cell::new(Lang::En));

    if dictionary.is_none() {
        window.set_status(SharedString::from(
            "Chưa có dữ liệu. Hãy đặt VIETDICT_DATA trỏ tới thư mục chứa dict.sqlite và *.fst.",
        ));
        window.set_detail(SharedString::from(
            "Bạn có thể build dữ liệu bằng dict-builder trước khi tra cứu.",
        ));
    }

    {
        let window_weak = window.as_weak();
        let dictionary = dictionary.clone();
        let lang = lang.clone();
        window.on_search(move |query| {
            let Some(window) = window_weak.upgrade() else {
                return;
            };
            let Some(dictionary) = dictionary.as_ref() else {
                window.set_suggestions("Chưa có dữ liệu từ điển.".into());
                return;
            };
            let query = query.to_string();
            if query.trim().is_empty() {
                window.set_suggestions("".into());
                window.set_detail("Nhập một từ để bắt đầu.".into());
                return;
            }
            let current = lang.get();
            let suggestions = dictionary.suggest(&query, current, 8);
            let labels: Vec<SharedString> = suggestions
                .iter()
                .map(|s| SharedString::from(format!("{} — {}", s.headword, s.preview)))
                .collect();
            window.set_suggestions(
                labels
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .into(),
            );
            match dictionary.lookup(&query, current) {
                Ok(Some(entry)) => {
                    let mut text = entry.headword.clone();
                    if let Some(ipa) = entry.ipa_us.as_deref() {
                        text.push_str(&format!("  {ipa}"));
                    }
                    for sense in entry.senses.iter().take(8) {
                        text.push_str("\n");
                        if let Some(pos) = sense.pos.as_deref() {
                            text.push_str(&format!("[{pos}] "));
                        }
                        text.push_str(&sense.gloss);
                    }
                    window.set_detail(text.into());
                }
                Ok(None) => {
                    window.set_detail("Không tìm thấy mục từ. Hãy thử một gợi ý bên trên.".into())
                }
                Err(error) => window.set_detail(format!("Lỗi tra cứu: {error}").into()),
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let lang = lang.clone();
        window.on_toggle_language(move || {
            let Some(window) = window_weak.upgrade() else {
                return;
            };
            let next = lang.get().other();
            lang.set(next);
            window.set_language_label(if next == Lang::En {
                "Anh → Việt".into()
            } else {
                "Việt → Anh".into()
            });
            window.set_suggestions("".into());
            window.set_detail(if next == Lang::En {
                "Nhập từ tiếng Anh.".into()
            } else {
                "Nhập từ tiếng Việt.".into()
            });
        });
    }

    window.show()?;
    slint::run_event_loop()?;
    Ok(())
}
