use regex::Regex;
use std::sync::LazyLock;

/// 非数字字符匹配
pub static NON_DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\D").unwrap());

/// 手机号匹配（支持 +86 前缀和分隔符）
pub static PHONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:^|\D)
        (?P<phone>
            (?:\+?86[-\s]?)?
            1[3-9]\d
            [-\s]?
            \d{4}
            [-\s]?
            \d{4}
        )
        (?:$|\D)
        ",
    )
        .unwrap()
});

/// 身份证号匹配
pub static ID_CARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:^|\D)
        (?P<id_card>
            [1-9]\d{5}
            (?:19|20)\d{2}
            (?:0[1-9]|1[0-2])
            (?:0[1-9]|[12]\d|3[01])
            \d{3}
            [\dXx]
        )
        (?:$|[^\dXx])
        ",
    )
        .unwrap()
});

/// 银行卡号匹配（16-19位）
pub static BANK_CARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?:^|\D)
        (?P<bank_card>
            \d{4}[-\s]?
            \d{4}[-\s]?
            \d{4}[-\s]?
            \d{4}
            (?:[-\s]?\d{1,3})?
        )
        (?:$|\D)
        ",
    )
        .unwrap()
});

pub const ID_WEIGHTS: [i32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
pub const ID_CHECK_CODES: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

pub fn clean_digits(s: &str) -> String {
    NON_DIGIT.replace_all(s, "").into_owned()
}

pub fn extract_phones(text: &str) -> Vec<(&str, usize, usize)> {
    PHONE
        .captures_iter(text)
        .filter_map(|c| c.name("phone").map(|m| (m.as_str(), m.start(), m.end())))
        .collect()
}

pub fn extract_id_cards(text: &str) -> Vec<(&str, usize, usize)> {
    ID_CARD
        .captures_iter(text)
        .filter_map(|c| c.name("id_card").map(|m| (m.as_str(), m.start(), m.end())))
        .collect()
}

pub fn extract_bank_cards(text: &str) -> Vec<(&str, usize, usize)> {
    BANK_CARD
        .captures_iter(text)
        .filter_map(|c| {
            c.name("bank_card")
                .map(|m| (m.as_str(), m.start(), m.end()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone() {
        assert!(PHONE.is_match("13812345678"));
        assert!(PHONE.is_match("+86 138 1234 5678"));
        assert!(!PHONE.is_match("12812345678"));
    }

    #[test]
    fn phone_chinese() {
        let r = extract_phones("联系13812345678请拨打");
        assert_eq!(r[0].0, "13812345678");
    }

    #[test]
    fn id_card() {
        assert!(ID_CARD.is_match("11010519900307888X"));
        assert!(!ID_CARD.is_match("11010519901307888X"));
    }

    #[test]
    fn id_card_chinese() {
        let r = extract_id_cards("身份证11010519900307888X核实");
        assert_eq!(r[0].0, "11010519900307888X");
    }

    #[test]
    fn bank_card() {
        assert!(BANK_CARD.is_match("6225880123456789"));
        assert!(BANK_CARD.is_match("6225 8801 2345 6789"));
        assert!(!BANK_CARD.is_match("622588012345"));
    }

    #[test]
    fn bank_card_chinese() {
        let r = extract_bank_cards("卡号6225880123456789绑定");
        assert_eq!(r[0].0, "6225880123456789");
    }

    #[test]
    fn clean() {
        assert_eq!(clean_digits("138-1234-5678"), "13812345678");
        assert_eq!(clean_digits("6225 8801 2345 6789"), "6225880123456789");
    }
}
