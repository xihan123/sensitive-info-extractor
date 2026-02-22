use eframe::egui;
use egui::{Color32, FontId, RichText, Vec2};

pub struct DragArea;

impl Default for DragArea {
    fn default() -> Self {
        Self
    }
}

impl DragArea {
    pub fn new() -> Self {
        Self
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<Vec<std::path::PathBuf>> {
        let mut dropped_paths = None;

        let is_dragging_file = ui.ctx().input(|i| {
            !i.raw.hovered_files.is_empty()
        });

        let background_color = if is_dragging_file {
            Color32::from_rgb(0xE3, 0xF2, 0xFD)
        } else {
            Color32::from_rgb(0xF5, 0xF5, 0xF5)
        };

        let border_color = if is_dragging_file {
            Color32::from_rgb(0x21, 0x96, 0xF3)
        } else {
            Color32::from_rgb(0xDD, 0xDD, 0xDD)
        };

        egui::Frame::default()
            .fill(background_color)
            .corner_radius(10.0)
            .stroke(egui::Stroke::new(2.0, border_color))
            .inner_margin(Vec2::new(40.0, 40.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if is_dragging_file {
                        ui.label(
                            RichText::new("📥 释放以添加文件")
                                .font(FontId::proportional(20.0))
                                .color(Color32::from_rgb(0x21, 0x96, 0xF3))
                        );
                    } else {
                        ui.label(
                            RichText::new("📁 拖拽文件或文件夹到此处")
                                .font(FontId::proportional(18.0))
                                .color(Color32::from_rgb(0x66, 0x66, 0x66))
                        );
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new("支持 .xlsx 文件")
                                .color(Color32::from_rgb(0x99, 0x99, 0x99))
                        );
                    }
                });
            });

        ui.ctx().input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<std::path::PathBuf> = i.raw.dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect();
                if !paths.is_empty() {
                    dropped_paths = Some(paths);
                }
            }
        });

        dropped_paths
    }
}