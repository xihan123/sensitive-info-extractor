use sensitive_info_extractor::core::InfoExtractor;
use sensitive_info_extractor::models::Config;
use sensitive_info_extractor::utils::{extract_bank_cards, extract_id_cards};

fn create_extractor() -> InfoExtractor {
    InfoExtractor::new(Config::default())
}

#[test]
fn test_extract_phone_numbers() {
    let extractor = create_extractor();
    let text = "联系方式：13812345678，备用：15912345678";

    let (phones, _, _, _) = extractor.extract(text);

    assert_eq!(phones.len(), 2);
    assert!(phones.iter().all(|p| p.is_valid));
}

#[test]
fn test_extract_id_cards() {
    let extractor = create_extractor();
    let text = "身份证号：440308199901010012";

    let raw = extract_id_cards(text);
    eprintln!("raw id_cards: {:?}", raw);

    let (_, id_cards, _, _) = extractor.extract(text);

    eprintln!("extracted id_cards: {:?}", id_cards);

    assert_eq!(id_cards.len(), 1);
    assert!(id_cards[0].is_valid);
}

#[test]
fn test_extract_bank_cards() {
    let extractor = create_extractor();
    let text = "银行卡号：4111111111111111";

    let raw = extract_bank_cards(text);
    eprintln!("raw bank_cards: {:?}", raw);

    let (_, _, bank_cards, _) = extractor.extract(text);

    eprintln!("extracted bank_cards: {:?}", bank_cards);

    assert!(!bank_cards.is_empty());
}

#[test]
fn test_extract_empty_text() {
    let extractor = create_extractor();
    let text = "";

    let (phones, id_cards, bank_cards, names) = extractor.extract(text);

    assert!(phones.is_empty());
    assert!(id_cards.is_empty());
    assert!(bank_cards.is_empty());
    assert!(names.is_empty());
}

#[test]
fn test_extract_no_sensitive_info() {
    let extractor = create_extractor();
    let text = "这是一段普通文字，没有任何敏感信息。";

    let (phones, id_cards, bank_cards, names) = extractor.extract(text);

    assert!(phones.is_empty());
    assert!(id_cards.is_empty());
    assert!(bank_cards.is_empty());
    assert!(names.is_empty());
}

#[test]
fn test_config_phone_only() {
    let mut config = Config::default();
    config.enable_phone = true;
    config.enable_id_card = false;
    config.enable_bank_card = false;
    config.enable_name = false;

    let extractor = InfoExtractor::new(config);
    let text = "电话13812345678";

    let (phones, id_cards, bank_cards, names) = extractor.extract(text);

    assert_eq!(phones.len(), 1);
    assert!(id_cards.is_empty());
    assert!(bank_cards.is_empty());
    assert!(names.is_empty());
}

#[test]
fn test_config_id_card_only() {
    let mut config = Config::default();
    config.enable_phone = false;
    config.enable_id_card = true;
    config.enable_bank_card = false;
    config.enable_name = false;

    let extractor = InfoExtractor::new(config);
    let text = "身份证号：440308199901010012";  // 使用冒号分隔以满足单词边界

    let raw = extract_id_cards(text);
    eprintln!("raw id_cards: {:?}", raw);

    let (phones, id_cards, bank_cards, names) = extractor.extract(text);

    eprintln!("id_cards: {:?}", id_cards);

    assert!(phones.is_empty());
    assert_eq!(id_cards.len(), 1);
    assert!(bank_cards.is_empty());
    assert!(names.is_empty());
}

#[test]
fn test_config_bank_card_only() {
    let mut config = Config::default();
    config.enable_phone = false;
    config.enable_id_card = false;
    config.enable_bank_card = true;
    config.enable_name = false;

    let extractor = InfoExtractor::new(config);
    let text = "银行卡号：4111111111111111";  // 使用冒号分隔以满足单词边界

    let raw = extract_bank_cards(text);
    eprintln!("raw bank_cards: {:?}", raw);

    let (phones, id_cards, bank_cards, names) = extractor.extract(text);

    eprintln!("bank_cards: {:?}", bank_cards);

    assert!(phones.is_empty());
    assert!(id_cards.is_empty());
    assert!(!bank_cards.is_empty());
    assert!(names.is_empty());
}

#[test]
fn test_config_all_disabled() {
    let mut config = Config::default();
    config.enable_phone = false;
    config.enable_id_card = false;
    config.enable_bank_card = false;
    config.enable_name = false;

    let extractor = InfoExtractor::new(config);
    let text = "电话13812345678";

    let (phones, id_cards, bank_cards, names) = extractor.extract(text);

    assert!(phones.is_empty());
    assert!(id_cards.is_empty());
    assert!(bank_cards.is_empty());
    assert!(names.is_empty());
}

#[test]
fn test_match_info_position() {
    let extractor = create_extractor();
    let text = "电话13812345678";

    let (phones, _, _, _) = extractor.extract(text);

    assert_eq!(phones.len(), 1);
    let phone = &phones[0];

    assert!(phone.position.0 < phone.position.1);
    assert_eq!(text[phone.position.0..phone.position.1], phone.value);
}

#[test]
fn test_config_name_disabled_by_default() {
    let config = Config::default();
    assert!(!config.enable_name);

    let extractor = InfoExtractor::new(config);
    let text = "张三和李四参加会议";

    let (_, _, _, names) = extractor.extract(text);

    // enable_name 默认为 false，所以应该返回空
    assert!(names.is_empty());
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