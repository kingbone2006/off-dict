mod clean;
mod parse;
mod record;
mod sqlite;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "dict-builder", about = "Build dữ liệu VietDict offline")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build SQLite + FST từ JSONL.
    Sqlite {
        #[arg(required = true)]
        input: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "data-local")]
        version: String,
    },
    /// Parse file OVDP `.tab` thành JSONL chuẩn.
    Parse {
        /// Nguồn: hongocduc/ovdp.
        #[arg(long, default_value = "hongocduc")]
        source: String,
        /// Ngôn ngữ mục từ: en hoặc vi.
        #[arg(long)]
        lang: String,
        /// File `.tab` đầu vào.
        #[arg(long)]
        input: PathBuf,
        /// File JSONL đầu ra.
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Sqlite {
            input,
            out,
            version,
        } => {
            let n = sqlite::build_from_jsonl(&input, &out, &version)?;
            println!("built {n} records in {}", out.display());
        }
        Command::Parse {
            source,
            lang,
            input,
            out,
        } => {
            anyhow::ensure!(
                source == "hongocduc" || source == "ovdp",
                "nguồn không hỗ trợ: {source}"
            );
            anyhow::ensure!(lang == "en" || lang == "vi", "lang phải là en hoặc vi");
            let (records, stats) = parse::parse_file(&input, &lang)
                .with_context(|| format!("parse {}", input.display()))?;
            let n = record::write_jsonl(&out, records.iter())?;
            println!("source={source} lines={} records={} skipped={} fixed_mojibake={} fixed_stray={} out={} records", stats.lines, stats.records, stats.skipped, stats.fixed_mojibake, stats.fixed_stray, n);
        }
    }
    Ok(())
}
