// 在 release 模式下隐藏控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod gui;
mod models;
mod utils;

use eframe::egui;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_resizable(true)
            .with_title("敏感信息提取工具"),
        ..Default::default()
    };

    eframe::run_native(
        "敏感信息提取工具",
        options,
        Box::new(|cc| {
            Ok(Box::new(gui::MainWindow::new(cc)))
        }),
    )
}
