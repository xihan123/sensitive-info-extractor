use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn is_xlsx_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("xlsx"))
        .unwrap_or(false)
}

pub fn scan_xlsx_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    scan_xlsx_files_recursive(dir, &mut files)?;

    files.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .cmp(b.file_name().unwrap_or_default())
    });

    Ok(files)
}

fn scan_xlsx_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("无法读取目录: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }
            scan_xlsx_files_recursive(&path, files)?;
        } else if is_xlsx_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

pub fn generate_output_filename_with_source(source_name: &str) -> String {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    format!("{}_{}.xlsx", source_name, timestamp)
}

pub fn process_dropped_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut xlsx_files = Vec::new();

    for path in paths {
        if path.is_dir() {
            let files = scan_xlsx_files(path)?;
            xlsx_files.extend(files);
        } else if is_xlsx_file(path) {
            xlsx_files.push(path.clone());
        }
    }

    xlsx_files.sort();
    xlsx_files.dedup();

    Ok(xlsx_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_xlsx_file() {
        assert!(is_xlsx_file(Path::new("test.xlsx")));
        assert!(is_xlsx_file(Path::new("test.XLSX")));
        assert!(!is_xlsx_file(Path::new("test.xls")));
        assert!(!is_xlsx_file(Path::new("test.txt")));
    }

    #[test]
    fn test_generate_output_filename_with_source() {
        let filename = generate_output_filename_with_source("测试文件");
        assert!(filename.starts_with("测试文件_"));
        assert!(filename.ends_with(".xlsx"));
    }
}