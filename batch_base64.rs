// batch_base64.rs — Rust версия

use rayon::prelude::*;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;
use base64::{Engine as _, engine::general_purpose};

struct BatchBase64 {
    input_dir: PathBuf,
    output_dir: PathBuf,
    threads: usize,
    decode: bool,
    recursive: bool,
    report: bool,
    processed: usize,
    errors: usize,
    failed: Vec<(String, String)>,
}

impl BatchBase64 {
    fn new(input_dir: &str, output_dir: Option<&str>, threads: usize, decode: bool, recursive: bool, report: bool) -> Self {
        let input_path = PathBuf::from(input_dir);
        let output_path = output_dir.map_or_else(
            || input_path.join("base64_output"),
            |d| PathBuf::from(d)
        );
        BatchBase64 {
            input_dir: input_path,
            output_dir: output_path,
            threads,
            decode,
            recursive,
            report,
            processed: 0,
            errors: 0,
            failed: Vec::new(),
        }
    }

    fn get_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if self.recursive {
            for entry in WalkDir::new(&self.input_dir) {
                if let Ok(entry) = entry {
                    if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        } else {
            if let Ok(entries) = fs::read_dir(&self.input_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        files.push(entry.path());
                    }
                }
            }
        }
        files
    }

    fn process_file(&mut self, file_path: &Path) -> Result<(), String> {
        let rel_path = file_path.strip_prefix(&self.input_dir).unwrap_or(file_path);
        let rel_str = rel_path.to_str().unwrap_or("");

        let output_path = if self.decode {
            let name = if rel_str.ends_with(".b64") {
                &rel_str[..rel_str.len()-4]
            } else {
                rel_str
            };
            self.output_dir.join(name)
        } else {
            self.output_dir.join(format!("{}.b64", rel_str))
        };

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Ошибка создания папки: {}", e))?;
        }

        if self.decode {
            let content = fs::read_to_string(file_path).map_err(|e| format!("Ошибка чтения: {}", e))?;
            let decoded = general_purpose::STANDARD.decode(content.trim()).map_err(|e| format!("Ошибка декодирования: {}", e))?;
            fs::write(output_path, decoded).map_err(|e| format!("Ошибка записи: {}", e))?;
        } else {
            let data = fs::read(file_path).map_err(|e| format!("Ошибка чтения: {}", e))?;
            let encoded = general_purpose::STANDARD.encode(&data);
            fs::write(output_path, encoded).map_err(|e| format!("Ошибка записи: {}", e))?;
        }

        Ok(())
    }

    fn run(&mut self) {
        println!("\x1b[36m🔐 Base64 Batch Encoder (Rust)\x1b[0m");
        println!("📁 Папка: {}", self.input_dir.display());

        let files = self.get_files();
        println!("📂 Найдено {} файлов", files.len());
        println!("⚡ Параллельная обработка ({} потоков)...\n", self.threads);

        let start = Instant::now();

        // Используем rayon для параллельной обработки
        let results: Vec<(PathBuf, Result<(), String>)> = files
            .par_iter()
            .map(|file| {
                let mut result = (file.clone(), Ok(()));
                if let Err(e) = self.process_file(file) {
                    result.1 = Err(e);
                }
                result
            })
            .collect();

        for (file, res) in results {
            match res {
                Ok(()) => { self.processed += 1; }
                Err(e) => {
                    self.errors += 1;
                    let rel = file.strip_prefix(&self.input_dir).unwrap_or(&file);
                    self.failed.push((rel.to_str().unwrap_or("").to_string(), e));
                }
            }
        }

        let elapsed = start.elapsed().as_secs_f64();

        println!();
        println!("\x1b[32m✅ Обработано: {} файлов\x1b[0m", self.processed);
        println!("\x1b[33m⚠️ Ошибок: {}\x1b[0m", self.errors);
        println!("\x1b[36m⏱️ Время: {:.2} сек\x1b[0m", elapsed);
        println!("\x1b[32m💾 Сохранено: {}\x1b[0m", self.output_dir.display());

        if self.report {
            self.save_report(elapsed);
        }
    }

    fn save_report(&self, elapsed: f64) {
        let report = json!({
            "input_dir": self.input_dir,
            "output_dir": self.output_dir,
            "total_files": self.processed + self.errors,
            "processed": self.processed,
            "errors": self.errors,
            "failed": self.failed.iter().map(|(p, e)| json!({"path": p, "error": e})).collect::<Vec<_>>(),
            "start_time": chrono::Local::now().to_rfc3339(),
            "end_time": chrono::Local::now().to_rfc3339(),
            "elapsed_seconds": elapsed,
            "decode_mode": self.decode
        });
        let report_path = self.output_dir.join("report.json");
        let json_str = serde_json::to_string_pretty(&report).unwrap();
        fs::write(report_path, json_str).unwrap();
        println!("\x1b[32m📊 Отчёт: {}/report.json\x1b[0m", self.output_dir.display());
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_dir = None;
    let mut output_dir = None;
    let mut threads = rayon::current_num_threads();
    let mut decode = false;
    let mut recursive = true;
    let mut report = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => { output_dir = Some(args[i+1].clone()); i += 2; }
            "--threads" | "-t" => { threads = args[i+1].parse().unwrap_or(threads); i += 2; }
            "--decode" | "-d" => { decode = true; i += 1; }
            "--no-recursive" => { recursive = false; i += 1; }
            "--no-report" => { report = false; i += 1; }
            _ => {
                if input_dir.is_none() && !args[i].starts_with("-") {
                    input_dir = Some(args[i].clone());
                }
                i += 1;
            }
        }
    }

    if input_dir.is_none() {
        println!("Usage: cargo run -- <input_dir> [--output dir] [--threads N] [--decode]");
        std::process::exit(1);
    }

    let input = input_dir.unwrap();
    if !Path::new(&input).exists() {
        println!("\x1b[31m❌ Папка не найдена: {}\x1b[0m", input);
        std::process::exit(1);
    }

    let mut batch = BatchBase64::new(&input, output_dir.as_deref(), threads, decode, recursive, report);
    batch.run();
}
