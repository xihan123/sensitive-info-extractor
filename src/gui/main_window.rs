use eframe::egui;
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, RichText, TextStyle};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use crate::core::{ExcelInfo, ProcessingStatistics, Processor};
use crate::models::{Config, ExtractResult, FileInfo, FileStatus};
use crate::utils::{generate_output_filename_with_source, process_dropped_paths};

enum ProcessingMessage {
    Progress(String, u8),
    Completed(Vec<ExtractResult>, ProcessingStatistics),
}

use super::{smart_select_column, ColumnSelector, DragArea, FileList, SettingsPanel};

pub struct MainWindow {
    config: Config,
    files: Vec<FileInfo>,
    available_columns: Vec<String>,
    results: Vec<ExtractResult>,
    statistics: Option<ProcessingStatistics>,
    processing: bool,
    progress: u8,
    current_file: String,
    status_message: String,
    error_message: Option<String>,
    drag_area: DragArea,
    processing_receiver: Option<Receiver<ProcessingMessage>>,
    processing_handle: Option<JoinHandle<()>>,
    api_connection_status: Option<Result<String, String>>,
}

impl Default for MainWindow {
    fn default() -> Self {
        Self {
            config: Config::default(),
            files: Vec::new(),
            available_columns: Vec::new(),
            results: Vec::new(),
            statistics: None,
            processing: false,
            progress: 0,
            current_file: String::new(),
            status_message: "准备就绪 - 拖拽xlsx文件到窗口".to_string(),
            error_message: None,
            drag_area: DragArea::new(),
            processing_receiver: None,
            processing_handle: None,
            api_connection_status: None,
        }
    }
}

impl MainWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::setup_chinese_fonts(&cc.egui_ctx);
        Self::default()
    }

    fn setup_chinese_fonts(ctx: &egui::Context) {
        use std::sync::Arc;

        let font_data = Self::load_chinese_font();

        let mut fonts = FontDefinitions::default();

        if let Some(data) = font_data {
            fonts.font_data.insert(
                "chinese_font".to_owned(),
                Arc::new(FontData::from_owned(data)),
            );

            fonts.families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "chinese_font".to_owned());

            fonts.families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "chinese_font".to_owned());
        }

        ctx.set_fonts(fonts);

        ctx.all_styles_mut(|style| {
            let text_styles: BTreeMap<TextStyle, FontId> = [
                (TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional)),
                (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
                (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
                (TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
                (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            ]
                .into();
            style.text_styles = text_styles;
        });
    }

    fn load_chinese_font() -> Option<Vec<u8>> {
        #[cfg(windows)]
        {
            let font_paths = [
                r"C:\Windows\Fonts\msyh.ttc",
                r"C:\Windows\Fonts\simhei.ttf",
                r"C:\Windows\Fonts\simsun.ttc",
            ];

            for path in &font_paths {
                if let Ok(data) = std::fs::read(path) {
                    return Some(data);
                }
            }
        }

        None
    }

    fn handle_dropped_files(&mut self, paths: &[PathBuf]) {
        match process_dropped_paths(paths) {
            Ok(xlsx_files) => {
                let mut added_count = 0;
                for path in xlsx_files {
                    if !self.files.iter().any(|f| f.file_path == path) {
                        let mut file_info = FileInfo::from_path(path);

                        match ExcelInfo::from_file(&file_info.file_path) {
                            Ok(info) => {
                                if let Some(columns) = info.first_sheet_columns() {
                                    file_info.columns = columns.clone();
                                    for col in columns {
                                        if !self.available_columns.contains(col) {
                                            self.available_columns.push(col.clone());
                                        }
                                    }
                                }
                                file_info.row_count = info.total_row_count() as u32;
                            }
                            Err(e) => {
                                file_info.status = FileStatus::error(e.to_string());
                            }
                        }
                        self.files.push(file_info);
                        added_count += 1;
                    }
                }

                smart_select_column(&self.available_columns, &mut self.config.target_column);

                if added_count > 0 {
                    self.status_message = format!("已导入 {} 个文件", added_count);
                    self.error_message = None;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("处理文件失败: {}", e));
            }
        }
    }

    fn start_processing(&mut self) {
        if self.files.is_empty() {
            self.error_message = Some("请先导入文件".to_string());
            return;
        }

        if !self.config.has_any_extraction_enabled() {
            self.error_message = Some("请至少选择一种提取类型".to_string());
            return;
        }

        let files_to_process: Vec<FileInfo> = self.files
            .iter()
            .filter(|f| f.selected && !f.status.is_error())
            .cloned()
            .collect();

        if files_to_process.is_empty() {
            self.error_message = Some("没有可处理的文件".to_string());
            return;
        }

        self.processing = true;
        self.error_message = None;
        self.status_message = "正在处理...".to_string();
        self.progress = 0;
        self.current_file.clear();
        self.results.clear();
        self.statistics = None;

        for file in &mut self.files {
            if file.selected {
                file.status = FileStatus::processing(0);
            }
        }

        let (sender, receiver) = mpsc::channel();
        self.processing_receiver = Some(receiver);

        let config = self.config.clone();

        let handle = thread::spawn(move || {
            let processor = Processor::new(config);

            // 克隆 sender 用于并行处理中的进度回调
            let sender_for_progress = sender.clone();

            // 使用 rayon 并行处理文件，返回结果和耗时
            let (results, elapsed_secs) = processor
                .process_files_parallel(&files_to_process, move |file_name, progress| {
                    let _ = sender_for_progress.send(ProcessingMessage::Progress(
                        file_name.to_string(),
                        progress,
                    ));
                });

            let mut all_results = Vec::new();
            for (file_name, result) in results {
                match result {
                    Ok(file_results) => {
                        all_results.extend(file_results);
                    }
                    Err(e) => {
                        tracing::error!("处理文件 {} 失败: {}", file_name, e);
                    }
                }
            }

            let stats = processor.generate_statistics(&all_results, elapsed_secs);
            let _ = sender.send(ProcessingMessage::Completed(all_results, stats));
        });

        self.processing_handle = Some(handle);
    }

    fn export_results(&mut self) {
        if self.results.is_empty() {
            self.error_message = Some("没有可导出的结果".to_string());
            return;
        }

        let source_name = self.results
            .first()
            .map(|r| r.source_file.clone())
            .unwrap_or_else(|| "result".to_string());

        let source_name = source_name.trim_end_matches(".xlsx").trim_end_matches(".XLSX");

        let output_path = std::env::current_dir()
            .unwrap_or_default()
            .join(generate_output_filename_with_source(source_name));

        let processor = Processor::new(self.config.clone());

        match processor.export_results(&self.results, &output_path) {
            Ok(()) => {
                self.status_message = format!("结果已导出到: {}", output_path.display());
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("导出失败: {}", e));
            }
        }
    }

    fn clear_all(&mut self) {
        self.files.clear();
        self.available_columns.clear();
        self.results.clear();
        self.statistics = None;
        self.config = Config::default();
        self.status_message = "已清空".to_string();
        self.error_message = None;
        self.processing_receiver = None;
        self.processing_handle = None;
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let receiver = self.processing_receiver.take();
        if let Some(rx) = receiver {
            let mut should_restore = true;
            let mut completed = false;

            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ProcessingMessage::Progress(file_name, progress) => {
                        self.current_file = file_name;
                        self.progress = progress;
                    }
                    ProcessingMessage::Completed(results, stats) => {
                        self.results = results;
                        let elapsed_str = if stats.elapsed_secs >= 60.0 {
                            let mins = (stats.elapsed_secs / 60.0).floor() as u32;
                            let secs = (stats.elapsed_secs % 60.0) as u32;
                            format!("{}分{}秒", mins, secs)
                        } else {
                            format!("{:.2}秒", stats.elapsed_secs)
                        };
                        self.statistics = Some(stats.clone());
                        self.processing = false;
                        self.progress = 100;
                        self.status_message = format!(
                            "提取完成，共 {} 条结果 (敏感信息: {} 条)，耗时 {}",
                            self.results.len(),
                            stats.total_sensitive_info(),
                            elapsed_str
                        );

                        for file in &mut self.files {
                            if file.selected {
                                file.status = FileStatus::completed();
                            }
                        }

                        should_restore = false;
                        completed = true;
                    }
                }
            }

            if should_restore && !completed {
                self.processing_receiver = Some(rx);
            }

            if !should_restore {
                self.processing_handle = None;
            }
        }

        if self.processing {
            ctx.request_repaint();
        }

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<PathBuf> = i.raw.dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect();
                if !paths.is_empty() {
                    self.handle_dropped_files(&paths);
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("敏感信息提取工具");
            });
            ui.separator();

            if let Some(paths) = self.drag_area.show(ui) {
                self.handle_dropped_files(&paths);
            }

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("📂 选择文件").clicked() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter("Excel", &["xlsx"])
                        .pick_files()
                    {
                        self.handle_dropped_files(&paths);
                    }
                }
                if ui.button("📁 选择文件夹").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.handle_dropped_files(&[path]);
                    }
                }
                if ui.button("🗑 清空").clicked() {
                    self.clear_all();
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);

                    FileList::new(&mut self.files).show(ui);

                    ui.add_space(10.0);

                    ColumnSelector::new(&self.available_columns, &mut self.config.target_column).show(ui);

                    ui.add_space(10.0);

                    SettingsPanel::new(&mut self.config, &mut self.api_connection_status).show(ui);
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.set_min_width(500.0);

                    ui.group(|ui| {
                        ui.heading("提取结果摘要");

                        if self.results.is_empty() {
                            ui.label("暂无结果 - 点击【开始处理】提取敏感信息");
                        } else if let Some(stats) = &self.statistics {
                            // 显示耗时
                            let elapsed_str = if stats.elapsed_secs >= 60.0 {
                                let mins = (stats.elapsed_secs / 60.0).floor() as u32;
                                let secs = (stats.elapsed_secs % 60.0) as u32;
                                format!("{}分{}秒", mins, secs)
                            } else {
                                format!("{:.2}秒", stats.elapsed_secs)
                            };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("⏱ 耗时: {}", elapsed_str)).strong());
                            });
                            ui.label(format!("共 {} 条结果", stats.total_results));
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("手机号:");
                                ui.label(format!("{} 个 (有效 {})", stats.total_phones, stats.valid_phones));
                            });
                            ui.horizontal(|ui| {
                                ui.label("身份证号:");
                                ui.label(format!("{} 个 (有效 {})", stats.total_id_cards, stats.valid_id_cards));
                            });
                            ui.horizontal(|ui| {
                                ui.label("银行卡号:");
                                ui.label(format!("{} 个 (有效 {})", stats.total_bank_cards, stats.valid_bank_cards));
                            });
                            if stats.total_names > 0 {
                                ui.horizontal(|ui| {
                                    ui.label("姓名:");
                                    ui.label(format!("{} 个 (可信 {})", stats.total_names, stats.valid_names));
                                });
                            }
                        }
                    });
                });
            });

            ui.add_space(10.0);

            if self.processing || self.progress > 0 {
                ui.horizontal(|ui| {
                    ui.label("进度:");
                    let available_width = ui.available_width().min(300.0);
                    let progress = egui::ProgressBar::new(self.progress as f32 / 100.0)
                        .text(format!("{}%", self.progress.min(100)))
                        .desired_width(available_width);
                    ui.add(progress);
                });
            }

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let process_enabled = !self.files.is_empty()
                    && !self.processing
                    && self.config.has_any_extraction_enabled();

                if ui.add_enabled(process_enabled, egui::Button::new("▶ 开始处理")).clicked() {
                    self.start_processing();
                }

                let export_enabled = !self.results.is_empty() && !self.processing;
                if ui.add_enabled(export_enabled, egui::Button::new("💾 导出结果")).clicked() {
                    self.export_results();
                }
            });

            ui.add_space(5.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                if let Some(err) = &self.error_message {
                    ui.label(RichText::new(err).color(Color32::from_rgb(0xF4, 0x43, 0x36)));
                }
            });
        });
    }
}