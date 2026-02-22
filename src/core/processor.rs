use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use rust_xlsxwriter::FormatBorder;
use rust_xlsxwriter::*;
use std::path::Path;
use std::sync::Arc;

use super::{ExcelReader, InfoExtractor};
use crate::models::{Config, ExtractResult, FileInfo};

pub type ProgressCallback = Box<dyn Fn(&str, u8) + Send + Sync>;

pub struct Processor {
    config: Config,
    progress_callback: Option<Arc<ProgressCallback>>,
}

impl Processor {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            progress_callback: None,
        }
    }

    /// 并行处理多个文件
    pub fn process_files_parallel(
        &self,
        files: &[FileInfo],
        progress_callback: impl Fn(&str, u8) + Sync + Send + 'static,
    ) -> Vec<(String, Result<Vec<ExtractResult>>)> {
        let total = files.len();
        let callback = Arc::new(progress_callback);

        files
            .par_iter()
            .enumerate()
            .map(|(index, file_info)| {
                let progress = ((index as f32 / total as f32) * 100.0) as u8;
                callback(&file_info.file_name, progress);

                let result = self.process_file(file_info);
                (file_info.file_name.clone(), result)
            })
            .collect()
    }

    pub fn process_file(&self, file_info: &FileInfo) -> Result<Vec<ExtractResult>> {
        let mut reader = ExcelReader::open(&file_info.file_path)
            .with_context(|| format!("无法打开文件: {}", file_info.file_name))?;

        let extractor = InfoExtractor::new(self.config.clone());
        let mut all_results = Vec::new();

        let sheet_names = reader.sheet_names();
        let total_sheets = sheet_names.len();

        for (sheet_index, sheet_name) in sheet_names.iter().enumerate() {
            if let Some(callback) = &self.progress_callback {
                let progress = ((sheet_index as f32 / total_sheets as f32) * 100.0) as u8;
                callback(&format!("{} - {}", file_info.file_name, sheet_name), progress);
            }

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

                let (phones, id_cards, bank_cards) = extractor.extract(&cell_value);

                if !phones.is_empty() || !id_cards.is_empty() || !bank_cards.is_empty() {
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

                    all_results.push(result);
                }
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
        const HEADERS: [&str; 12] = [
            "源文件名", "工作表", "行号", "手机号", "手机号有效性",
            "身份证号", "身份证有效性", "银行卡号", "银行卡有效性",
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

        worksheet.write_string(row, 9, &result.source_text)?;
        worksheet.write_string(row, 10, result.context_before_str())?;
        worksheet.write_string(row, 11, result.context_after_str())?;

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
        const COLUMN_WIDTHS: [(u16, f64); 12] = [
            (0, 20.0), (1, 15.0), (2, 8.0), (3, 20.0), (4, 12.0),
            (5, 22.0), (6, 12.0), (7, 22.0), (8, 12.0),
            (9, 50.0), (10, 30.0), (11, 30.0),
        ];

        for (col, width) in COLUMN_WIDTHS {
            worksheet.set_column_width(col, width)?;
        }

        worksheet.set_freeze_panes(1, 0)?;
        worksheet.autofilter(0, 0, 0, 11)?;

        Ok(())
    }

    pub fn generate_statistics(&self, results: &[ExtractResult]) -> ProcessingStatistics {
        ProcessingStatistics {
            total_results: results.len(),
            total_phones: results.iter().map(|r| r.phone_numbers.len()).sum(),
            valid_phones: results.iter().flat_map(|r| &r.phone_numbers).filter(|m| m.is_valid).count(),
            total_id_cards: results.iter().map(|r| r.id_cards.len()).sum(),
            valid_id_cards: results.iter().flat_map(|r| &r.id_cards).filter(|m| m.is_valid).count(),
            total_bank_cards: results.iter().map(|r| r.bank_cards.len()).sum(),
            valid_bank_cards: results.iter().flat_map(|r| &r.bank_cards).filter(|m| m.is_valid).count(),
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
}

impl ProcessingStatistics {
    pub fn total_sensitive_info(&self) -> usize {
        self.total_phones + self.total_id_cards + self.total_bank_cards
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

        assert_eq!(stats.total_sensitive_info(), 28);
    }
}