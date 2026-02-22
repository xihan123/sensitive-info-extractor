use eframe::egui;
use egui::{Color32, RichText};

pub struct ColumnSelector<'a> {
    available_columns: &'a [String],
    selected_column: &'a mut String,
}

impl<'a> ColumnSelector<'a> {
    pub fn new(available_columns: &'a [String], selected_column: &'a mut String) -> Self {
        Self {
            available_columns,
            selected_column,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("目标列:");

            if self.available_columns.is_empty() {
                ui.label(
                    RichText::new("(导入文件后显示可用列)")
                        .small()
                        .color(Color32::GRAY)
                );
            } else {
                egui::ComboBox::from_id_salt("column_selector")
                    .selected_text(&*self.selected_column)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for col in self.available_columns {
                            let is_recommended = col.contains("消息内容");

                            if is_recommended {
                                ui.selectable_value(
                                    self.selected_column,
                                    col.clone(),
                                    RichText::new(format!("⭐ {} (推荐)", col)),
                                );
                            } else {
                                ui.selectable_value(
                                    self.selected_column,
                                    col.clone(),
                                    col,
                                );
                            }
                        }
                    });

                ui.label(
                    RichText::new(format!("({} 列可用)", self.available_columns.len()))
                        .small()
                        .color(Color32::GRAY)
                );
            }
        });
    }
}

pub fn find_recommended_column(columns: &[String]) -> Option<&String> {
    columns.iter().find(|col| col.contains("消息内容"))
}

pub fn smart_select_column(columns: &[String], current_selection: &mut String) {
    if !current_selection.is_empty() && columns.contains(current_selection) {
        return;
    }

    if let Some(recommended) = find_recommended_column(columns) {
        *current_selection = recommended.clone();
        return;
    }

    if let Some(first) = columns.first() {
        if !first.is_empty() {
            *current_selection = first.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_recommended_column() {
        let columns = vec![
            "姓名".to_string(),
            "消息内容".to_string(),
            "时间".to_string(),
        ];

        let recommended = find_recommended_column(&columns);
        assert_eq!(recommended, Some(&"消息内容".to_string()));
    }

    #[test]
    fn test_smart_select_column() {
        let columns = vec![
            "姓名".to_string(),
            "消息内容".to_string(),
        ];

        let mut selected = String::new();
        smart_select_column(&columns, &mut selected);
        assert_eq!(selected, "消息内容");

        selected = "姓名".to_string();
        smart_select_column(&columns, &mut selected);
        assert_eq!(selected, "姓名");
    }
}