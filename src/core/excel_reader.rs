use anyhow::{Context, Result};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use std::collections::HashMap;
use std::path::Path;

pub struct ExcelReader {
    workbook: Xlsx<std::io::BufReader<std::fs::File>>,
}

impl ExcelReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let file_path = path_ref.to_string_lossy().to_string();

        let workbook: Xlsx<_> = open_workbook(path_ref)
            .with_context(|| format!("无法打开Excel文件: {}", file_path))?;

        Ok(Self { workbook })
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.workbook.sheet_names().to_vec()
    }

    pub fn read_sheet(&mut self, sheet_name: &str) -> Result<SheetData> {
        let range = self.workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("无法读取工作表: {}", sheet_name))?;

        let rows = Self::range_to_rows(&range);

        Ok(SheetData {
            rows,
        })
    }

    fn range_to_rows(range: &Range<Data>) -> Vec<Vec<String>> {
        let mut rows = Vec::new();

        let start = range.start().unwrap_or((0, 0));
        let end = range.end().unwrap_or((0, 0));

        for row in start.0..=end.0 {
            let mut row_data = Vec::new();
            for col in start.1..=end.1 {
                let cell_value = range
                    .get_value((row, col))
                    .map(Self::data_to_string)
                    .unwrap_or_default();
                row_data.push(cell_value);
            }
            rows.push(row_data);
        }

        rows
    }

    fn data_to_string(data: &Data) -> String {
        match data {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{}", *f as i64)
                } else {
                    format!("{}", f)
                }
            }
            Data::Int(i) => format!("{}", i),
            Data::Bool(b) => format!("{}", b),
            Data::DateTime(dt) => format!("{}", dt),
            Data::Error(e) => format!("{:?}", e),
            _ => String::new(),
        }
    }

    pub fn read_column_names(&mut self, sheet_name: &str) -> Result<Vec<String>> {
        let range = self.workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("无法读取工作表: {}", sheet_name))?;

        let mut columns = Vec::new();

        if let Some((_, end_col)) = range.end() {
            for col in 0..=end_col {
                let cell_value = range
                    .get_value((0, col))
                    .map(Self::data_to_string)
                    .unwrap_or_default();
                columns.push(cell_value);
            }
        }

        Ok(columns)
    }

    pub fn row_count(&mut self, sheet_name: &str) -> Result<usize> {
        let range = self.workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("无法读取工作表: {}", sheet_name))?;

        let start = range.start().unwrap_or((0, 0));
        let end = range.end().unwrap_or((0, 0));

        let count = if end.0 >= start.0 {
            (end.0 - start.0) as usize
        } else {
            0
        };

        Ok(count)
    }
}

#[derive(Debug, Clone)]
pub struct SheetData {
    pub rows: Vec<Vec<String>>,
}

impl SheetData {
    pub fn column_names(&self) -> Vec<String> {
        self.rows.first().cloned().unwrap_or_default()
    }

    pub fn get_column_index(&self, column_name: &str) -> Option<usize> {
        self.rows.first()?.iter().position(|c| c == column_name)
    }

    pub fn get_column_by_name(&self, column_name: &str) -> Result<Vec<(usize, String)>> {
        let col_index = self.get_column_index(column_name)
            .with_context(|| format!("列不存在: {}", column_name))?;

        let mut result = Vec::new();

        for (row_index, row) in self.rows.iter().enumerate().skip(1) {
            if col_index < row.len() {
                result.push((row_index, row[col_index].clone()));
            } else {
                result.push((row_index, String::new()));
            }
        }

        Ok(result)
    }

    pub fn get_context(&self, row_index: usize, context_lines: usize) -> (Vec<String>, Vec<String>) {
        let mut before = Vec::new();
        let mut after = Vec::new();

        for i in (1..=context_lines).rev() {
            let idx = row_index + 1;
            if idx > i {
                if let Some(row) = self.rows.get(idx - i) {
                    before.push(row.join(" | "));
                }
            }
        }

        for i in 1..=context_lines {
            let idx = row_index + 1 + i;
            if let Some(row) = self.rows.get(idx) {
                after.push(row.join(" | "));
            }
        }

        (before, after)
    }
}

#[derive(Debug, Clone)]
pub struct ExcelInfo {
    pub sheet_names: Vec<String>,
    pub sheet_columns: HashMap<String, Vec<String>>,
    pub sheet_row_counts: HashMap<String, usize>,
}

impl ExcelInfo {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();

        let mut reader = ExcelReader::open(path_ref)?;
        let sheet_names = reader.sheet_names();

        let mut sheet_columns = HashMap::new();
        let mut sheet_row_counts = HashMap::new();

        for sheet_name in &sheet_names {
            let columns = reader.read_column_names(sheet_name)?;
            let row_count = reader.row_count(sheet_name)?;

            sheet_columns.insert(sheet_name.clone(), columns);
            sheet_row_counts.insert(sheet_name.clone(), row_count);
        }

        Ok(Self {
            sheet_names,
            sheet_columns,
            sheet_row_counts,
        })
    }

    pub fn first_sheet_columns(&self) -> Option<&Vec<String>> {
        self.sheet_names
            .first()
            .and_then(|name| self.sheet_columns.get(name))
    }

    pub fn total_row_count(&self) -> usize {
        self.sheet_row_counts.values().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sheet_data_column_names() {
        let sheet_data = SheetData {
            rows: vec![
                vec!["姓名".to_string(), "消息内容".to_string()],
                vec!["张三".to_string(), "电话13812345678".to_string()],
            ],
        };

        let columns = sheet_data.column_names();
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0], "姓名");
        assert_eq!(columns[1], "消息内容");
    }

    #[test]
    fn test_sheet_data_get_column_index() {
        let sheet_data = SheetData {
            rows: vec![
                vec!["姓名".to_string(), "消息内容".to_string()],
                vec!["张三".to_string(), "电话13812345678".to_string()],
            ],
        };

        assert_eq!(sheet_data.get_column_index("姓名"), Some(0));
        assert_eq!(sheet_data.get_column_index("消息内容"), Some(1));
        assert_eq!(sheet_data.get_column_index("不存在"), None);
    }
}