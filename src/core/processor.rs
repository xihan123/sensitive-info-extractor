use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use rust_xlsxwriter::FormatBorder;
use rust_xlsxwriter::*;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::{ExcelReader, InfoExtractor};
use crate::models::{Config, ExtractResult, FileInfo};

pub struct Processor {
    config: Config,
}

impl Processor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// 并行处理多个文件（基于行数计算进度）
    pub fn process_files_parallel(
        &self,
        files: &[FileInfo],
        progress_callback: impl Fn(&str, u8) + Sync + Send + 'static,
    ) -> (Vec<(String, Result<Vec<ExtractResult>>)>, f64) {
        let start_time = Instant::now();
        let callback = Arc::new(progress_callback);

        // 计算所有文件的总行数
        let total_rows: usize = files.iter().map(|f| f.row_count as usize).sum();
        if total_rows == 0 {
            callback("准备处理", 0);
            let results: Vec<(String, Result<Vec<ExtractResult>>)> = files
                .iter()
                .map(|file_info| {
                    let result = self.process_file_with_progress(file_info, None);
                    (file_info.file_name.clone(), result)
                })
                .collect();
            callback("处理完成", 100);
            let elapsed = start_time.elapsed().as_secs_f64();
            return (results, elapsed);
        }

        let processed_rows = Arc::new(AtomicUsize::new(0));

        callback("准备处理", 0);

        let results: Vec<(String, Result<Vec<ExtractResult>>)> = files
            .par_iter()
            .map(|file_info| {
                // 为每个文件创建进度回调闭包
                let callback_clone = Arc::clone(&callback);
                let processed_rows_clone = Arc::clone(&processed_rows);
                let total = total_rows;

                let file_progress_callback = move |rows_processed: usize, current_file: &str| {
                    let total_processed = processed_rows_clone.fetch_add(rows_processed, Ordering::SeqCst) + rows_processed;
                    let progress = ((total_processed as f64 / total as f64) * 100.0).min(100.0) as u8;
                    callback_clone(current_file, progress);
                };

                let result = self.process_file_with_progress(file_info, Some(&file_progress_callback));
                (file_info.file_name.clone(), result)
            })
            .collect();

        callback("处理完成", 100);
        let elapsed = start_time.elapsed().as_secs_f64();
        (results, elapsed)
    }

    /// 处理单个文件（支持行级进度回调）
    fn process_file_with_progress(
        &self,
        file_info: &FileInfo,
        progress_callback: Option<&dyn Fn(usize, &str)>,
    ) -> Result<Vec<ExtractResult>> {
        let mut reader = ExcelReader::open(&file_info.file_path)
            .with_context(|| format!("无法打开文件: {}", file_info.file_name))?;

        let extractor = InfoExtractor::new(self.config.clone());
        let mut all_results = Vec::new();
        let mut rows_processed = 0usize;
        // 动态计算更新间隔：总行数的1%或最少100行
        let update_interval = ((file_info.row_count as usize) / 100).max(100).min(500);

        let sheet_names = reader.sheet_names();

        for sheet_name in &sheet_names {
            let sheet_data = reader.read_sheet(sheet_name)?;

            let target_column = if self.config.target_column.is_empty() {
                self.find_target_column(&sheet_data)?
            } else {
                self.config.target_column.clone()
            };

            let column_data = match sheet_data.get_column_by_name(&target_column) {
                Ok(data) => data,
                Err(_) => continue,
            };

            for (row_index, cell_value) in column_data {
                if cell_value.is_empty() {
                    continue;
                }

                let (phones, id_cards, bank_cards, names) = extractor.extract(&cell_value);

                if !phones.is_empty() || !id_cards.is_empty() || !bank_cards.is_empty() || !names.is_empty() {
                    let (context_before, context_after) = sheet_data
                        .get_context(row_index, self.config.context_lines as usize);

                    let mut result = ExtractResult::new(
                        &file_info.file_name,
                        sheet_name,
                        (row_index + 1) as u32,
                    );

                    result.source_text = cell_value;
                    result.context_before = context_before;
                    result.context_after = context_after;
                    result.phone_numbers = phones;
                    result.id_cards = id_cards;
                    result.bank_cards = bank_cards;
                    result.names = names;

                    all_results.push(result);
                }

                rows_processed += 1;
                // 定期更新进度
                if rows_processed >= update_interval {
                    if let Some(cb) = progress_callback {
                        cb(rows_processed, &file_info.file_name);
                    }
                    rows_processed = 0;
                }
            }
        }

        // 处理剩余的行
        if rows_processed > 0 {
            if let Some(cb) = progress_callback {
                cb(rows_processed, &file_info.file_name);
            }
        }

        Ok(all_results)
    }

    fn find_target_column(&self, sheet_data: &crate::core::excel_reader::SheetData) -> Result<String> {
        let columns = sheet_data.column_names();

        for col in &columns {
            if col.contains("消息内容") {
                return Ok(col.clone());
            }
        }

        columns.first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("工作表没有可用的列"))
    }

    pub fn export_results(&self, results: &[ExtractResult], output_path: &Path) -> Result<()> {
        if results.is_empty() {
            bail!("没有可导出的结果");
        }

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        self.write_headers(worksheet)?;

        for (row_index, result) in results.iter().enumerate() {
            let row = row_index as u32 + 1;
            self.write_result_row(worksheet, row, result)?;
        }

        self.apply_formatting(worksheet)?;

        workbook.save(output_path)
            .with_context(|| format!("无法保存文件: {}", output_path.display()))?;

        tracing::info!("结果已导出到: {}", output_path.display());
        Ok(())
    }

    fn write_headers(&self, worksheet: &mut Worksheet) -> Result<()> {
        const HEADERS: [&str; 14] = [
            "源文件名", "工作表", "行号", "手机号", "手机号有效性",
            "身份证号", "身份证有效性", "银行卡号", "银行卡有效性",
            "姓名", "姓名有效性",
            "源文本", "上文", "下文",
        ];

        let header_format = Format::new()
            .set_bold()
            .set_background_color("#4472C4")
            .set_font_color(Color::White)
            .set_border(FormatBorder::Thin);

        for (col, header) in HEADERS.iter().enumerate() {
            worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
        }

        Ok(())
    }

    fn write_result_row(&self, worksheet: &mut Worksheet, row: u32, result: &ExtractResult) -> Result<()> {
        let valid_format = Format::new().set_font_color(Color::Green);
        let invalid_format = Format::new().set_font_color(Color::Red);

        worksheet.write_string(row, 0, &result.source_file)?;
        worksheet.write_string(row, 1, &result.sheet_name)?;
        worksheet.write_number(row, 2, result.row_number)?;
        worksheet.write_string(row, 3, result.phone_numbers_str())?;

        Self::write_validity_cell(worksheet, row, 4, &result.phone_validity_str(), &valid_format, &invalid_format)?;

        worksheet.write_string(row, 5, result.id_cards_str())?;
        Self::write_validity_cell(worksheet, row, 6, &result.id_card_validity_str(), &valid_format, &invalid_format)?;

        worksheet.write_string(row, 7, result.bank_cards_str())?;
        Self::write_validity_cell(worksheet, row, 8, &result.bank_card_validity_str(), &valid_format, &invalid_format)?;

        worksheet.write_string(row, 9, result.names_str())?;
        Self::write_validity_cell(worksheet, row, 10, &result.names_validity_str(), &valid_format, &invalid_format)?;

        worksheet.write_string(row, 11, &result.source_text)?;
        worksheet.write_string(row, 12, result.context_before_str())?;
        worksheet.write_string(row, 13, result.context_after_str())?;

        Ok(())
    }

    fn write_validity_cell(
        worksheet: &mut Worksheet,
        row: u32,
        col: u16,
        validity: &str,
        valid_format: &Format,
        invalid_format: &Format,
    ) -> Result<()> {
        if validity.contains("无效") {
            worksheet.write_string_with_format(row, col, validity, invalid_format)?;
        } else if !validity.is_empty() {
            worksheet.write_string_with_format(row, col, validity, valid_format)?;
        } else {
            worksheet.write_string(row, col, "")?;
        }
        Ok(())
    }

    fn apply_formatting(&self, worksheet: &mut Worksheet) -> Result<()> {
        const COLUMN_WIDTHS: [(u16, f64); 14] = [
            (0, 20.0), (1, 15.0), (2, 8.0), (3, 20.0), (4, 12.0),
            (5, 22.0), (6, 12.0), (7, 22.0), (8, 12.0),
            (9, 15.0), (10, 12.0),
            (11, 50.0), (12, 30.0), (13, 30.0),
        ];

        for (col, width) in COLUMN_WIDTHS {
            worksheet.set_column_width(col, width)?;
        }

        worksheet.set_freeze_panes(1, 0)?;
        worksheet.autofilter(0, 0, 0, 13)?;

        Ok(())
    }

    pub fn generate_statistics(&self, results: &[ExtractResult], elapsed_secs: f64) -> ProcessingStatistics {
        ProcessingStatistics {
            total_results: results.len(),
            total_phones: results.iter().map(|r| r.phone_numbers.len()).sum(),
            valid_phones: results.iter().flat_map(|r| &r.phone_numbers).filter(|m| m.is_valid).count(),
            total_id_cards: results.iter().map(|r| r.id_cards.len()).sum(),
            valid_id_cards: results.iter().flat_map(|r| &r.id_cards).filter(|m| m.is_valid).count(),
            total_bank_cards: results.iter().map(|r| r.bank_cards.len()).sum(),
            valid_bank_cards: results.iter().flat_map(|r| &r.bank_cards).filter(|m| m.is_valid).count(),
            total_names: results.iter().map(|r| r.names.len()).sum(),
            valid_names: results.iter().flat_map(|r| &r.names).filter(|m| m.is_valid).count(),
            elapsed_secs,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProcessingStatistics {
    pub total_results: usize,
    pub total_phones: usize,
    pub valid_phones: usize,
    pub total_id_cards: usize,
    pub valid_id_cards: usize,
    pub total_bank_cards: usize,
    pub valid_bank_cards: usize,
    pub total_names: usize,
    pub valid_names: usize,
    pub elapsed_secs: f64,
}

impl ProcessingStatistics {
    pub fn total_sensitive_info(&self) -> usize {
        self.total_phones + self.total_id_cards + self.total_bank_cards + self.total_names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_statistics() {
        let mut stats = ProcessingStatistics::default();
        stats.total_results = 10;
        stats.total_phones = 20;
        stats.valid_phones = 18;
        stats.total_id_cards = 5;
        stats.valid_id_cards = 5;
        stats.total_bank_cards = 3;
        stats.valid_bank_cards = 2;
        stats.total_names = 8;
        stats.valid_names = 7;

        assert_eq!(stats.total_sensitive_info(), 36);
    }
}
