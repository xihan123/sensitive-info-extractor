use super::validator::Validator;
use super::NameExtractor;
use crate::models::{Config, MatchInfo};
use crate::utils::{extract_bank_cards, extract_id_cards, extract_phones};

pub struct InfoExtractor {
    config: Config,
    name_extractor: NameExtractor,
}

impl InfoExtractor {
    pub fn new(config: Config) -> Self {
        let name_extractor = NameExtractor::with_batch_size(
            config.api_host.clone(),
            config.enable_name,
            config.batch_size,
        );
        Self { config, name_extractor }
    }

    #[allow(dead_code)]
    pub fn extract(&self, text: &str) -> (Vec<MatchInfo>, Vec<MatchInfo>, Vec<MatchInfo>, Vec<MatchInfo>) {
        let (phones, id_cards, bank_cards) = self.extract_local(text);
        let names = if self.config.enable_name {
            self.name_extractor.extract(text)
        } else {
            Vec::new()
        };
        (phones, id_cards, bank_cards, names)
    }

    pub fn extract_local(&self, text: &str) -> (Vec<MatchInfo>, Vec<MatchInfo>, Vec<MatchInfo>) {
        let phones = if self.config.enable_phone {
            self.extract_phones(text)
        } else {
            Vec::new()
        };

        let id_cards = if self.config.enable_id_card {
            self.extract_id_cards(text)
        } else {
            Vec::new()
        };

        let valid_id_card_positions: Vec<(usize, usize)> = id_cards
            .iter()
            .filter(|m| m.is_valid)
            .map(|m| m.position)
            .collect();

        let bank_cards = if self.config.enable_bank_card {
            self.extract_bank_cards_filtered(text, &valid_id_card_positions)
        } else {
            Vec::new()
        };

        (phones, id_cards, bank_cards)
    }

    pub fn extract_names_batch(&self, texts: &[&str]) -> Vec<Vec<MatchInfo>> {
        if self.config.enable_name {
            self.name_extractor.extract_batch(texts)
        } else {
            vec![Vec::new(); texts.len()]
        }
    }

    fn extract_bank_cards_filtered(&self, text: &str, exclude_positions: &[(usize, usize)]) -> Vec<MatchInfo> {
        extract_bank_cards(text)
            .into_iter()
            .filter(|(_, start, end)| {
                !exclude_positions.iter().any(|(id_start, id_end)| {
                    *start < *id_end && *end > *id_start
                })
            })
            .map(|(value, start, end)| {
                let is_valid = Validator::validate_bank_card(&value);
                MatchInfo::new(value, is_valid, start, end)
            })
            .collect()
    }

    fn extract_phones(&self, text: &str) -> Vec<MatchInfo> {
        extract_phones(text)
            .into_iter()
            .map(|(value, start, end)| {
                let is_valid = Validator::validate_phone(&value);
                MatchInfo::new(value, is_valid, start, end)
            })
            .collect()
    }

    fn extract_id_cards(&self, text: &str) -> Vec<MatchInfo> {
        extract_id_cards(text)
            .into_iter()
            .map(|(value, start, end)| {
                let is_valid = Validator::validate_id_card(&value);
                MatchInfo::new(value, is_valid, start, end)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_extractor() -> InfoExtractor {
        InfoExtractor::new(Config::default())
    }

    #[test]
    fn test_extract_phones() {
        let extractor = create_extractor();
        let text = "联系方式：13812345678，备用：15912345678";
        let (phones, _, _, _) = extractor.extract(text);

        assert_eq!(phones.len(), 2);
        assert!(phones[0].is_valid);
        assert!(phones[1].is_valid);
    }

    #[test]
    fn test_extract_id_cards() {
        let extractor = create_extractor();
        let text = "身份证号：440308199901010012";
        let (_, id_cards, _, _) = extractor.extract(text);

        assert_eq!(id_cards.len(), 1);
        assert!(id_cards[0].is_valid);
    }

    #[test]
    fn test_extract_bank_cards() {
        let extractor = create_extractor();
        let text = "银行卡：4111111111111111";
        let (_, _, bank_cards, _) = extractor.extract(text);

        assert_eq!(bank_cards.len(), 1);
        assert!(bank_cards[0].is_valid);
    }

    #[test]
    fn test_valid_id_card_not_matched_as_bank_card() {
        let extractor = create_extractor();
        let text = "身份证：110105199003072039";
        let (_, id_cards, bank_cards, _) = extractor.extract(text);

        assert_eq!(id_cards.len(), 1);
        assert!(id_cards[0].is_valid);

        assert_eq!(bank_cards.len(), 0);
    }

    #[test]
    fn test_invalid_id_card_can_be_matched_as_bank_card() {
        let extractor = create_extractor();
        let text = "号码：110105199003072030";
        let (_, id_cards, bank_cards, _) = extractor.extract(text);

        assert_eq!(id_cards.len(), 1);
        assert!(!id_cards[0].is_valid);

        assert!(!bank_cards.is_empty());
    }
}
