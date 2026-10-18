use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordRuleViolation {
    LengthOutOfRange,
    MissingUppercase,
    MissingLowercase,
    MissingDigit,
    ContainsForbiddenChar(char),
    ContainsForbiddenLetter(char),
    ContainsUsername,
}

impl std::fmt::Display for PasswordRuleViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LengthOutOfRange => write!(f, "Şifre 6 ile 10 karakter arasında olmalıdır"),
            Self::MissingUppercase => write!(f, "En az bir büyük harf içermelidir"),
            Self::MissingLowercase => write!(f, "En az bir küçük harf içermelidir"),
            Self::MissingDigit => write!(f, "En az bir rakam içermelidir"),
            Self::ContainsForbiddenChar(c) => write!(f, "İzin verilmeyen karakter içeriyor: '{c}'"),
            Self::ContainsForbiddenLetter(c) => write!(f, "İzin verilmeyen harf içeriyor: '{c}'"),
            Self::ContainsUsername => {
                write!(f, "Şifre kullanıcı adı veya kimlik numarası içeremez")
            }
        }
    }
}

impl PasswordRuleViolation {
    pub fn localized_message(&self, is_tr: bool) -> String {
        if is_tr {
            self.to_string()
        } else {
            match self {
                Self::LengthOutOfRange => {
                    "Password must be between 6 and 10 characters long".to_string()
                }
                Self::MissingUppercase => {
                    "Password must contain at least one uppercase letter".to_string()
                }
                Self::MissingLowercase => {
                    "Password must contain at least one lowercase letter".to_string()
                }
                Self::MissingDigit => "Password must contain at least one digit".to_string(),
                Self::ContainsForbiddenChar(c) => {
                    format!("Forbidden character: '{c}'")
                }
                Self::ContainsForbiddenLetter(c) => {
                    format!("Forbidden letter: '{c}'")
                }
                Self::ContainsUsername => {
                    "Password must not contain username or ID number".to_string()
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PasswordRuleStatus {
    pub length_ok: bool,
    pub chars_ok: bool,
    pub no_forbidden_chars: bool,
    pub no_forbidden_letters: bool,
    pub no_username: bool,
}

impl PasswordRuleStatus {
    pub fn is_all_valid(&self) -> bool {
        self.length_ok
            && self.chars_ok
            && self.no_forbidden_chars
            && self.no_forbidden_letters
            && self.no_username
    }
}

pub fn check_password_rules(password: &str, username: Option<&str>) -> PasswordRuleStatus {
    let char_count = password.chars().count();
    let length_ok = (6..=10).contains(&char_count);

    let forbidden_chars = ['\'', '|', '{', '}', '(', ')', '[', ']'];
    let forbidden_letters = [
        'İ', 'ı', 'Ö', 'ö', 'Ü', 'ü', 'Ç', 'ç', 'Ğ', 'ğ', 'Ş', 'ş', '£',
    ];

    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_digit = false;
    let mut no_forbidden_chars = true;
    let mut no_forbidden_letters = true;

    for c in password.chars() {
        if forbidden_chars.contains(&c) {
            no_forbidden_chars = false;
        }
        if forbidden_letters.contains(&c) {
            no_forbidden_letters = false;
        }
        if c.is_ascii_uppercase() {
            has_upper = true;
        } else if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        }
    }

    let chars_ok = has_upper && has_lower && has_digit;

    let no_username = if password.is_empty() {
        false
    } else if let Some(user) = username {
        let trimmed = user.trim();
        trimmed.is_empty() || !password.contains(trimmed)
    } else {
        true
    };

    PasswordRuleStatus {
        length_ok,
        chars_ok,
        no_forbidden_chars: !password.is_empty() && no_forbidden_chars,
        no_forbidden_letters: !password.is_empty() && no_forbidden_letters,
        no_username,
    }
}

pub fn validate_password(
    password: &str,
    username: Option<&str>,
) -> Result<(), PasswordRuleViolation> {
    let char_count = password.chars().count();
    if !(6..=10).contains(&char_count) {
        return Err(PasswordRuleViolation::LengthOutOfRange);
    }

    let forbidden_chars = ['\'', '|', '{', '}', '(', ')', '[', ']'];
    let forbidden_letters = [
        'İ', 'ı', 'Ö', 'ö', 'Ü', 'ü', 'Ç', 'ç', 'Ğ', 'ğ', 'Ş', 'ş', '£',
    ];

    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_digit = false;

    for c in password.chars() {
        if forbidden_chars.contains(&c) {
            return Err(PasswordRuleViolation::ContainsForbiddenChar(c));
        }
        if forbidden_letters.contains(&c) {
            return Err(PasswordRuleViolation::ContainsForbiddenLetter(c));
        }
        if c.is_ascii_uppercase() {
            has_upper = true;
        } else if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        }
    }

    if !has_upper {
        return Err(PasswordRuleViolation::MissingUppercase);
    }
    if !has_lower {
        return Err(PasswordRuleViolation::MissingLowercase);
    }
    if !has_digit {
        return Err(PasswordRuleViolation::MissingDigit);
    }

    if let Some(user) = username {
        let trimmed_user = user.trim();
        if !trimmed_user.is_empty() && password.contains(trimmed_user) {
            return Err(PasswordRuleViolation::ContainsUsername);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordChangeRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub username: String,
    pub temporary_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Nationality {
    #[default]
    Tc,
    Foreigner,
}

impl Nationality {
    pub fn form_value(&self) -> &'static str {
        match self {
            Self::Tc => "TC",
            Self::Foreigner => "YB",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UserType {
    #[default]
    Student,
    AdministrativeStaff,
    YouthCenterUser,
    YouthCenterStaff,
    TohmUser,
    SemUser,
}

impl UserType {
    pub fn form_value(&self) -> &'static str {
        match self {
            Self::Student => "Ogrenci",
            Self::AdministrativeStaff => "IdariPersonel",
            Self::YouthCenterUser => "GenclikMerkeziKullanicisi",
            Self::YouthCenterStaff => "GenclikMerkeziPersoneli",
            Self::TohmUser => "TohmKullanicisi",
            Self::SemUser => "SemKullanicisi",
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            0 => Self::Student,
            1 => Self::AdministrativeStaff,
            2 => Self::YouthCenterUser,
            3 => Self::YouthCenterStaff,
            4 => Self::TohmUser,
            5 => Self::SemUser,
            _ => Self::Student,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationRequest {
    pub nationality: Nationality,
    pub user_type: UserType,
    pub identity_number: String,
    pub first_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_password() {
        assert_eq!(validate_password("Gsbyurt1", Some("12345678901")), Ok(()));
        assert_eq!(validate_password("Aa1234", None), Ok(()));
    }

    #[test]
    fn test_length_out_of_range() {
        assert_eq!(
            validate_password("Aa1", None),
            Err(PasswordRuleViolation::LengthOutOfRange)
        );
        assert_eq!(
            validate_password("Aa123456789", None),
            Err(PasswordRuleViolation::LengthOutOfRange)
        );
    }

    #[test]
    fn test_missing_requirements() {
        assert_eq!(
            validate_password("abcdef", None),
            Err(PasswordRuleViolation::MissingUppercase)
        );
        assert_eq!(
            validate_password("ABCDEF", None),
            Err(PasswordRuleViolation::MissingLowercase)
        );
        assert_eq!(
            validate_password("Abcdef", None),
            Err(PasswordRuleViolation::MissingDigit)
        );
    }

    #[test]
    fn test_forbidden_characters() {
        assert_eq!(
            validate_password("Abc12{", None),
            Err(PasswordRuleViolation::ContainsForbiddenChar('{'))
        );
        assert_eq!(
            validate_password("Abc12ş", None),
            Err(PasswordRuleViolation::ContainsForbiddenLetter('ş'))
        );
        assert_eq!(
            validate_password("Abc12İ", None),
            Err(PasswordRuleViolation::ContainsForbiddenLetter('İ'))
        );
    }

    #[test]
    fn test_contains_username() {
        assert_eq!(
            validate_password("A123456b", Some("123456")),
            Err(PasswordRuleViolation::ContainsUsername)
        );
    }
}
