use crate::utils::{clean_digits, ID_CHECK_CODES, ID_WEIGHTS};

pub struct Validator;

impl Validator {
    pub fn validate_id_card(id_card: &str) -> bool {
        if id_card.len() != 18 {
            return false;
        }

        let chars: Vec<char> = id_card.chars().collect();

        // 检查前17位是否都是数字
        for &c in chars.iter().take(17) {
            if !c.is_ascii_digit() {
                return false;
            }
        }

        let last_char = chars[17];
        if !last_char.is_ascii_digit() && last_char != 'X' && last_char != 'x' {
            return false;
        }

        if !Self::verify_id_card_checksum(&chars) {
            return false;
        }

        Self::verify_id_card_birth_date(&chars)
    }

    fn verify_id_card_checksum(chars: &[char]) -> bool {
        let mut sum: i32 = 0;

        for i in 0..17 {
            let digit = match chars[i].to_digit(10) {
                Some(d) => d as i32,
                None => return false,
            };
            sum += digit * ID_WEIGHTS[i];
        }

        let remainder = (sum % 11) as usize;
        let expected_check_code = ID_CHECK_CODES[remainder];

        let last_char = chars[17].to_ascii_uppercase();
        last_char == expected_check_code
    }

    fn verify_id_card_birth_date(chars: &[char]) -> bool {
        let year_str: String = chars[6..10].iter().collect();
        let month_str: String = chars[10..12].iter().collect();
        let day_str: String = chars[12..14].iter().collect();

        let year = match year_str.parse::<u32>() {
            Ok(y) => y,
            Err(_) => return false,
        };
        let month = match month_str.parse::<u32>() {
            Ok(m) => m,
            Err(_) => return false,
        };
        let day = match day_str.parse::<u32>() {
            Ok(d) => d,
            Err(_) => return false,
        };

        if !(1900..=2099).contains(&year) {
            return false;
        }

        if !(1..=12).contains(&month) {
            return false;
        }

        let days_in_month = Self::days_in_month(year, month);
        day >= 1 && day <= days_in_month
    }

    fn days_in_month(year: u32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }

    pub fn validate_bank_card(card_number: &str) -> bool {
        let clean_number = clean_digits(card_number);

        let len = clean_number.len();
        if !(16..=19).contains(&len) {
            return false;
        }

        if !clean_number.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        Self::luhn_check(&clean_number)
    }

    fn luhn_check(number: &str) -> bool {
        let digits: Vec<u32> = number
            .chars()
            .filter_map(|c| c.to_digit(10))
            .collect();

        if digits.len() != number.len() {
            return false;
        }

        let len = digits.len();
        let mut sum: u32 = 0;

        for (i, &digit) in digits.iter().enumerate().rev() {
            let position_from_right = len - i;

            if position_from_right.is_multiple_of(2) {
                let doubled = digit * 2;
                sum += if doubled > 9 { doubled - 9 } else { doubled };
            } else {
                sum += digit;
            }
        }

        sum.is_multiple_of(10)
    }

    pub fn validate_phone(phone: &str) -> bool {
        let clean_number = clean_digits(phone);

        if clean_number.len() != 11 {
            return false;
        }

        if !clean_number.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        let mut chars = clean_number.chars();

        let first_char = match chars.next() {
            Some(c) => c,
            None => return false,
        };

        if first_char != '1' {
            return false;
        }

        let second_char = match chars.next() {
            Some(c) => c,
            None => return false,
        };

        matches!(second_char, '3'..='9')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_id_card() {
        // 110105199003072039 的校验码是正确的 (sum=190, 190%11=3, check_codes[3]='9')
        assert!(Validator::validate_id_card("110105199003072039"));

        // 无效的身份证号
        assert!(!Validator::validate_id_card("11010519900307")); // 长度不足
        assert!(!Validator::validate_id_card("110105199013072039")); // 无效月份
        assert!(!Validator::validate_id_card("110105199003322039")); // 无效日期
        assert!(!Validator::validate_id_card("11010519900307203Y")); // 无效校验码字符
        assert!(!Validator::validate_id_card("11010519900307203X")); // 校验码错误 (应该是9)
    }

    #[test]
    fn test_validate_bank_card() {
        assert!(Validator::validate_bank_card("4111111111111111"));
        assert!(Validator::validate_bank_card("5500000000000004"));
        assert!(Validator::validate_bank_card("4111 1111 1111 1111"));

        assert!(!Validator::validate_bank_card("6225880123456780"));
        assert!(!Validator::validate_bank_card("622588012345678"));
        assert!(!Validator::validate_bank_card("62258801234567890123"));
    }

    #[test]
    fn test_validate_phone() {
        assert!(Validator::validate_phone("13812345678"));
        assert!(Validator::validate_phone("138-1234-5678"));
        assert!(Validator::validate_phone("15912345678"));
        assert!(Validator::validate_phone("18612345678"));

        assert!(!Validator::validate_phone("12812345678"));
        assert!(!Validator::validate_phone("12345678"));
        assert!(!Validator::validate_phone("23812345678"));
    }

    #[test]
    fn test_luhn_check() {
        assert!(Validator::luhn_check("79927398713"));
        assert!(Validator::luhn_check("4111111111111111"));
        assert!(!Validator::luhn_check("79927398710"));
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(Validator::days_in_month(2020, 1), 31);
        assert_eq!(Validator::days_in_month(2020, 2), 29);
        assert_eq!(Validator::days_in_month(2021, 2), 28);
        assert_eq!(Validator::days_in_month(2020, 4), 30);
    }
}