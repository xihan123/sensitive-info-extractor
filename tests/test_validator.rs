use sensitive_info_extractor::core::validator::Validator;

#[test]
fn test_validate_id_card_valid() {
    assert!(Validator::validate_id_card("110105199003072039")); // 正确校验码
}

#[test]
fn test_validate_id_card_invalid_length() {
    assert!(!Validator::validate_id_card("11010519900307203"));
    assert!(!Validator::validate_id_card("11010519900307203XX"));
    assert!(!Validator::validate_id_card(""));
}

#[test]
fn test_validate_id_card_invalid_date() {
    assert!(!Validator::validate_id_card("110105199013072039"));
    assert!(!Validator::validate_id_card("110105199012322039"));
    assert!(!Validator::validate_id_card("110105199002302039"));
}

#[test]
fn test_validate_id_card_invalid_checksum() {
    assert!(!Validator::validate_id_card("110105199003072038")); // 校验码错误
}

#[test]
fn test_validate_bank_card_valid() {
    assert!(Validator::validate_bank_card("4111111111111111"));
    assert!(Validator::validate_bank_card("5500000000000004"));
}

#[test]
fn test_validate_bank_card_invalid_length() {
    assert!(!Validator::validate_bank_card("411111111111111"));
    assert!(!Validator::validate_bank_card("41111111111111111"));
}

#[test]
fn test_validate_bank_card_invalid_luhn() {
    assert!(!Validator::validate_bank_card("4111111111111112"));
    assert!(!Validator::validate_bank_card("5500000000000001"));
}

#[test]
fn test_validate_bank_card_with_spaces() {
    assert!(Validator::validate_bank_card("4111 1111 1111 1111"));
    assert!(Validator::validate_bank_card("5500 0000 0000 0004"));
}

#[test]
fn test_validate_phone_valid() {
    assert!(Validator::validate_phone("13812345678"));
    assert!(Validator::validate_phone("15912345678"));
    assert!(Validator::validate_phone("18612345678"));
    assert!(Validator::validate_phone("15012345678"));
    assert!(Validator::validate_phone("17012345678"));
    assert!(Validator::validate_phone("19012345678"));
}

#[test]
fn test_validate_phone_invalid() {
    assert!(!Validator::validate_phone("12812345678")); // 无效号段
    assert!(!Validator::validate_phone("23812345678")); // 第一位不是1
    assert!(!Validator::validate_phone("12345678")); // 长度不足
    assert!(!Validator::validate_phone("123456789012")); // 长度过长
    assert!(!Validator::validate_phone("")); // 空
}

#[test]
fn test_validate_phone_with_separators() {
    assert!(Validator::validate_phone("138-1234-5678"));
    assert!(Validator::validate_phone("138 1234 5678"));
}
