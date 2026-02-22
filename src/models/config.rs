use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub context_lines: u32,
    pub target_column: String,
    pub enable_phone: bool,
    pub enable_id_card: bool,
    pub enable_bank_card: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            context_lines: 2,
            target_column: "消息内容".to_string(),
            enable_phone: true,
            enable_id_card: true,
            enable_bank_card: true,
        }
    }
}

impl Config {
    pub fn has_any_extraction_enabled(&self) -> bool {
        self.enable_phone || self.enable_id_card || self.enable_bank_card
    }
}