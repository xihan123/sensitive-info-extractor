use crate::models::{FileInfo, FileStatus};
use eframe::egui;
use egui::{Color32, RichText};

pub struct FileList<'a> {
    files: &'a mut Vec<FileInfo>,
}

impl<'a> FileList<'a> {
    pub fn new(files: &'a mut Vec<FileInfo>) -> Self {
        Self { files }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("已选文件").strong());
                ui.label(format!("({})", self.files.len()));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("清空").clicked() {
                        self.files.clear();
                    }
                    if ui.small_button("取消全选").clicked() {
                        for file in self.files.iter_mut() {
                            file.selected = false;
                        }
                    }
                    if ui.small_button("全选").clicked() {
                        for file in self.files.iter_mut() {
                            file.selected = true;
                        }
                    }
                });
            });

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for file in self.files.iter_mut() {
                        Self::show_file_item(ui, file);
                    }

                    if self.files.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(RichText::new("暂无文件").color(Color32::GRAY));
                            ui.label(RichText::new("拖拽文件到上方区域添加").color(Color32::GRAY).small());
                        });
                    }
                });
        });
    }

    fn show_file_item(ui: &mut egui::Ui, file: &mut FileInfo) {
        egui::Frame::default()
            .inner_margin(egui::Vec2::new(5.0, 2.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut file.selected, "");

                    ui.label("📄");
                    ui.label(&file.file_name);

                    if file.row_count > 0 {
                        ui.label(
                            RichText::new(format!("({} 行)", file.row_count))
                                .small()
                                .color(Color32::GRAY)
                        );
                    }

                    Self::show_status_tag(ui, &file.status);
                });
            });
    }

    fn show_status_tag(ui: &mut egui::Ui, status: &FileStatus) {
        let text: String;
        let color: Color32;

        match status {
            FileStatus::Pending => {
                text = "等待处理".to_string();
                color = Color32::GRAY;
            }
            FileStatus::Processing(_) => {
                text = "处理中".to_string();
                color = Color32::from_rgb(0x21, 0x96, 0xF3);
            }
            FileStatus::Completed => {
                text = "已完成".to_string();
                color = Color32::from_rgb(0x4C, 0xAF, 0x50);
            }
            FileStatus::Error(msg) => {
                ui.label(
                    RichText::new(format!("❌ {}", msg))
                        .small()
                        .color(Color32::from_rgb(0xF4, 0x43, 0x36))
                );
                return;
            }
        }

        egui::Frame::default()
            .fill(color.linear_multiply(0.1))
            .corner_radius(4.0)
            .inner_margin(egui::Vec2::new(6.0, 2.0))
            .show(ui, |ui| {
                ui.label(RichText::new(text).small().color(color));
            });
    }
}
