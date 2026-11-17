//! Benchmark tra cứu. Cần dữ liệu thật: `just data` (hoặc đặt `VIETDICT_DATA`).

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, Criterion};
use vietdict_core::{Dictionary, Lang};

fn data_dir() -> PathBuf {
    std::env::var_os("VIETDICT_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/build"))
}

fn bench(c: &mut Criterion) {
    let dict = match Dictionary::open(&data_dir()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("bỏ qua benchmark: {e}");
            return;
        }
    };
    c.bench_function("open", |b| {
        b.iter(|| Dictionary::open(&data_dir()).map(|_| ()).is_ok())
    });
    c.bench_function("suggest_prefix_en", |b| {
        b.iter(|| dict.suggest(black_box("hap"), Lang::En, 20))
    });
    c.bench_function("suggest_exact_en", |b| {
        b.iter(|| dict.suggest(black_box("receive"), Lang::En, 20))
    });
    c.bench_function("suggest_fuzzy_en", |b| {
        b.iter(|| dict.suggest(black_box("recieve"), Lang::En, 20))
    });
    c.bench_function("suggest_vi_no_diacritics", |b| {
        b.iter(|| dict.suggest(black_box("con meo"), Lang::Vi, 20))
    });
    c.bench_function("lookup_en", |b| {
        b.iter(|| dict.lookup(black_box("run"), Lang::En))
    });
    c.bench_function("reverse_lookup", |b| {
        b.iter(|| dict.reverse_lookup(black_box("con mèo"), Lang::En, 20))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
