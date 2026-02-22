use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInfo {
    pub value: String,
    pub is_valid: bool,
    pub position: (usize, usize),
}

impl MatchInfo {
    pub fn new(value: impl Into<String>, is_valid: bool, start: usize, end: usize) -> Self {
        Self {
            value: value.into(),
            is_valid,
            position: (start, end),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub source_file: String,
    pub sheet_name: String,
    pub row_number: u32,
    pub phone_numbers: Vec<MatchInfo>,
    pub id_cards: Vec<MatchInfo>,
    pub bank_cards: Vec<MatchInfo>,
    pub source_text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

impl ExtractResult {
    pub fn new(
        source_file: impl Into<String>,
        sheet_name: impl Into<String>,
        row_number: u32,
    ) -> Self {
        Self {
            source_file: source_file.into(),
            sheet_name: sheet_name.into(),
            row_number,
            phone_numbers: Vec::new(),
            id_cards: Vec::new(),
            bank_cards: Vec::new(),
            source_text: String::new(),
            context_before: Vec::new(),
            context_after: Vec::new(),
        }
    }

    pub fn phone_numbers_str(&self) -> String {
        format_matches(&self.phone_numbers)
    }

    pub fn id_cards_str(&self) -> String {
        format_matches(&self.id_cards)
    }

    pub fn bank_cards_str(&self) -> String {
        format_matches(&self.bank_cards)
    }

    pub fn phone_validity_str(&self) -> String {
        format_validity(&self.phone_numbers)
    }

    pub fn id_card_validity_str(&self) -> String {
        format_validity(&self.id_cards)
    }

    pub fn bank_card_validity_str(&self) -> String {
        format_validity(&self.bank_cards)
    }

    pub fn context_before_str(&self) -> String {
        self.context_before.join("\n")
    }

    pub fn context_after_str(&self) -> String {
        self.context_after.join("\n")
    }
}

fn format_matches(matches: &[MatchInfo]) -> String {
    matches
        .iter()
        .map(|m| m.value.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_validity(matches: &[MatchInfo]) -> String {
    matches
        .iter()
        .map(|m| if m.is_valid { "有效" } else { "无效" })
        .collect::<Vec<_>>()
        .join(", ")
}