//! 언어와 다국어 텍스트.

use serde::{Deserialize, Serialize};

/// 화면 표시 언어. 기본값은 한국어다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    Ko,
    En,
}

impl Language {
    /// API와 옵션에서 쓰는 언어 코드("ko"/"en").
    pub fn code(&self) -> &'static str {
        match self {
            Language::Ko => "ko",
            Language::En => "en",
        }
    }

    /// 언어 코드를 해석한다. 대소문자는 구분하지 않는다.
    pub fn parse(s: &str) -> Option<Language> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ko" => Some(Language::Ko),
            "en" => Some(Language::En),
            _ => None,
        }
    }
}

/// 언어별 문자열 묶음.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LocalizedText {
    pub ko: Option<String>,
    pub en: Option<String>,
}

impl LocalizedText {
    /// 요청 언어 → 영어 순으로 찾는다. 둘 다 없으면 `None`.
    pub fn get(&self, lang: Language) -> Option<&str> {
        let preferred = match lang {
            Language::Ko => self.ko.as_deref(),
            Language::En => self.en.as_deref(),
        };
        preferred.or(self.en.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_prefers_requested_language() {
        let t = LocalizedText {
            ko: Some("피".into()),
            en: Some("P".into()),
        };
        assert_eq!(t.get(Language::Ko), Some("피"));
        assert_eq!(t.get(Language::En), Some("P"));
    }

    #[test]
    fn get_falls_back_to_english() {
        let t = LocalizedText {
            ko: None,
            en: Some("P".into()),
        };
        assert_eq!(t.get(Language::Ko), Some("P"));
    }

    #[test]
    fn get_none_when_both_missing() {
        assert_eq!(LocalizedText { ko: None, en: None }.get(Language::Ko), None);
    }

    #[test]
    fn parse_language() {
        assert_eq!(Language::parse("EN"), Some(Language::En));
        assert_eq!(Language::parse("fr"), None);
    }
}
