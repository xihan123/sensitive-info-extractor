use crate::core::NameExtractor;
use crate::models::{Config, BATCH_MAX_LIMIT, BATCH_RECOMMENDED_SIZE};
use eframe::egui;
use egui::{Color32, RichText};

#[derive(Clone, Default)]
pub struct NameTestResult {
    pub names: Vec<String>,
    pub confidence: f64,
    pub is_valid: bool,
}

pub struct SettingsPanel<'a> {
    config: &'a mut Config,
    connection_status: &'a mut Option<Result<String, String>>,
    test_input: &'a mut String,
    test_result: &'a mut Option<NameTestResult>,
}

impl<'a> SettingsPanel<'a> {
    pub fn new(
        config: &'a mut Config,
        connection_status: &'a mut Option<Result<String, String>>,
        test_input: &'a mut String,
        test_result: &'a mut Option<NameTestResult>,
    ) -> Self {
        Self {
            config,
            connection_status,
            test_input,
            test_result,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(RichText::new("提取设置").strong());

            self.show_context_lines_setting(ui);

            ui.add_space(8.0);

            self.show_extraction_types_setting(ui);

            ui.add_space(8.0);

            self.show_api_setting(ui);

            ui.add_space(8.0);

            self.show_name_test_panel(ui);

            ui.add_space(8.0);

            self.show_config_summary(ui);
        });
    }

    fn show_context_lines_setting(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("上下文行数:");

            let slider = egui::Slider::new(&mut self.config.context_lines, 0..=10)
                .text("行")
                .step_by(1.0);

            ui.add(slider);

            ui.label(
                RichText::new("（提取时包含的上下文行数）")
                    .small()
                    .color(Color32::GRAY)
            );
        });
    }

    fn show_extraction_types_setting(&mut self, ui: &mut egui::Ui) {
        ui.label("提取类型:");

        ui.horizontal_wrapped(|ui| {
            let phone_checkbox = ui.checkbox(&mut self.config.enable_phone, "📱 手机号");
            phone_checkbox.on_hover_text("匹配中国大陆手机号（11位，1开头）");

            let id_card_checkbox = ui.checkbox(&mut self.config.enable_id_card, "🪪 身份证号");
            id_card_checkbox.on_hover_text("匹配18位身份证号并验证校验码");

            let bank_card_checkbox = ui.checkbox(&mut self.config.enable_bank_card, "💳 银行卡号");
            bank_card_checkbox.on_hover_text("匹配16-19位银行卡号并验证Luhn校验");

            let name_checkbox = ui.checkbox(&mut self.config.enable_name, "👤 姓名");
            name_checkbox.on_hover_text("通过 API 服务提取姓名（需配置 API 地址）");
        });

        if !self.config.has_any_extraction_enabled() {
            ui.label(
                RichText::new("⚠ 请至少选择一种提取类型")
                    .small()
                    .color(Color32::from_rgb(0xFF, 0x98, 0x00))
            );
        }
    }

    fn show_api_setting(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("API 设置（姓名提取）")
            .default_open(self.config.enable_name)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("API 地址:");

                    ui.add_enabled(
                        self.config.enable_name,
                        egui::TextEdit::singleline(&mut self.config.api_host)
                            .desired_width(200.0)
                            .hint_text("localhost:8080"),
                    );

                    ui.label(
                        RichText::new("（姓名提取服务地址）")
                            .small()
                            .color(Color32::GRAY)
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("批量大小:");

                    let slider = egui::Slider::new(&mut self.config.batch_size, 1..=BATCH_MAX_LIMIT)
                        .text("条/批")
                        .step_by(1.0);

                    ui.add_enabled(self.config.enable_name, slider);

                    ui.label(
                        RichText::new(format!("（推荐: {}）", BATCH_RECOMMENDED_SIZE))
                            .small()
                            .color(Color32::GRAY)
                    );
                });

                ui.horizontal(|ui| {
                    let test_enabled = self.config.enable_name && !self.config.api_host.is_empty();

                    if ui.add_enabled(test_enabled, egui::Button::new("🔍 测试连接")).clicked() {
                        let extractor = NameExtractor::new(self.config.api_host.clone(), true);
                        *self.connection_status = Some(extractor.check_connection());
                    }

                    if let Some(status) = self.connection_status.as_ref() {
                        match status {
                            Ok(msg) => {
                                ui.label(RichText::new(format!("✓ {}", msg)).color(Color32::GREEN));
                            }
                            Err(err) => {
                                ui.label(RichText::new(format!("✗ {}", err)).color(Color32::RED));
                            }
                        }
                    }
                });

                if self.config.enable_name {
                    ui.label(
                        RichText::new("💡 提示: 姓名 API 需要运行服务端，地址格式: host:port")
                            .small()
                            .color(Color32::from_rgb(0x21, 0x96, 0xF3))
                    );
                }
            });
    }

    fn show_name_test_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("🧪 姓名提取测试")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(RichText::new("输入测试文本:").small());

                ui.add(
                    egui::TextEdit::multiline(self.test_input)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("例如：张三在北京工作，李四昨天出差了"),
                );

                ui.horizontal(|ui| {
                    let test_enabled = self.config.enable_name
                        && !self.config.api_host.is_empty()
                        && !self.test_input.trim().is_empty();

                    if ui.add_enabled(test_enabled, egui::Button::new("🔮 提取姓名")).clicked() {
                        let extractor = NameExtractor::new(self.config.api_host.clone(), true);
                        let results = extractor.extract(self.test_input.trim());

                        *self.test_result = Some(NameTestResult {
                            names: results.iter().map(|r| r.value.clone()).collect(),
                            confidence: if results.iter().all(|r| r.is_valid) { 1.0 } else { 0.8 },
                            is_valid: results.iter().all(|r| r.is_valid),
                        });
                    }

                    if ui.button("🗑 清空").clicked() {
                        self.test_input.clear();
                        *self.test_result = None;
                    }
                });

                if let Some(result) = self.test_result.as_ref() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new("提取结果:").strong());

                    if result.names.is_empty() {
                        ui.label(
                            RichText::new("未检测到姓名")
                                .color(Color32::from_rgb(0x9E, 0x9E, 0x9E))
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for name in &result.names {
                                ui.label(
                                    RichText::new(format!("👤 {}", name))
                                        .color(Color32::from_rgb(0x4C, 0xAF, 0x50))
                                        .strong()
                                );
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!(
                                "置信度: {:.0}%",
                                result.confidence * 100.0
                            )).small());

                            if result.is_valid {
                                ui.label(RichText::new("✓ 可信").small().color(Color32::GREEN));
                            } else {
                                ui.label(RichText::new("⚠ 待确认").small().color(Color32::YELLOW));
                            }
                        });
                    }
                }
            });
    }

    fn show_config_summary(&self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("当前配置摘要")
            .default_open(false)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!(
                        "• 目标列: {}",
                        if self.config.target_column.is_empty() {
                            "(自动选择)".to_string()
                        } else {
                            self.config.target_column.clone()
                        }
                    )).small());

                    ui.label(RichText::new(format!(
                        "• 上下文行数: {} 行",
                        self.config.context_lines
                    )).small());

                    let types: Vec<&str> = [
                        if self.config.enable_phone { Some("手机号") } else { None },
                        if self.config.enable_id_card { Some("身份证号") } else { None },
                        if self.config.enable_bank_card { Some("银行卡号") } else { None },
                        if self.config.enable_name { Some("姓名") } else { None },
                    ].iter().filter_map(|&x| x).collect();

                    ui.label(RichText::new(format!(
                        "• 提取类型: {}",
                        if types.is_empty() { "无".to_string() } else { types.join(", ") }
                    )).small());

                    if self.config.enable_name {
                        ui.label(RichText::new(format!(
                            "• API 地址: {}",
                            self.config.api_host
                        )).small());
                        ui.label(RichText::new(format!(
                            "• 批量大小: {} 条/批",
                            self.config.batch_size
                        )).small());
                    }
                });
            });
    }
}
