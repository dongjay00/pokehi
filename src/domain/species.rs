//! 도감 도메인 값 객체: 도감 번호, 타입, 도감 설명, 종.

use serde::{Deserialize, Serialize};

use super::language::{Language, LocalizedText};

/// 전국도감 번호. 1 이상이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PokedexNumber(pub u32);

impl PokedexNumber {
    /// 화면 표시용 라벨. 3자리 이상, 0으로 채운다(`No.025`, `No.1000`).
    pub fn label(&self) -> String {
        format!("No.{:03}", self.0)
    }
}

/// 포켓몬 타입 18종.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PokemonType {
    Normal,
    Fire,
    Water,
    Electric,
    Grass,
    Ice,
    Fighting,
    Poison,
    Ground,
    Flying,
    Psychic,
    Bug,
    Rock,
    Ghost,
    Dragon,
    Dark,
    Steel,
    Fairy,
}

impl PokemonType {
    /// 모든 타입. `sync`가 타입 이름을 받을 때 이 목록을 쓴다.
    pub const ALL: [PokemonType; 18] = [
        PokemonType::Normal,
        PokemonType::Fire,
        PokemonType::Water,
        PokemonType::Electric,
        PokemonType::Grass,
        PokemonType::Ice,
        PokemonType::Fighting,
        PokemonType::Poison,
        PokemonType::Ground,
        PokemonType::Flying,
        PokemonType::Psychic,
        PokemonType::Bug,
        PokemonType::Rock,
        PokemonType::Ghost,
        PokemonType::Dragon,
        PokemonType::Dark,
        PokemonType::Steel,
        PokemonType::Fairy,
    ];

    /// PokéAPI의 타입 이름(`type.name`).
    pub fn api_name(&self) -> &'static str {
        match self {
            PokemonType::Normal => "normal",
            PokemonType::Fire => "fire",
            PokemonType::Water => "water",
            PokemonType::Electric => "electric",
            PokemonType::Grass => "grass",
            PokemonType::Ice => "ice",
            PokemonType::Fighting => "fighting",
            PokemonType::Poison => "poison",
            PokemonType::Ground => "ground",
            PokemonType::Flying => "flying",
            PokemonType::Psychic => "psychic",
            PokemonType::Bug => "bug",
            PokemonType::Rock => "rock",
            PokemonType::Ghost => "ghost",
            PokemonType::Dragon => "dragon",
            PokemonType::Dark => "dark",
            PokemonType::Steel => "steel",
            PokemonType::Fairy => "fairy",
        }
    }

    /// API 이름으로 타입을 찾는다. 18종에 없는 이름(`stellar`, `unknown` 등)은 `None`.
    pub fn from_api_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.api_name() == name)
    }

    /// 타입 뱃지 배경색.
    pub fn badge_rgb(&self) -> (u8, u8, u8) {
        let hex: u32 = match self {
            PokemonType::Normal => 0xA8A77A,
            PokemonType::Fire => 0xEE8130,
            PokemonType::Water => 0x6390F0,
            PokemonType::Electric => 0xF7D02C,
            PokemonType::Grass => 0x7AC74C,
            PokemonType::Ice => 0x96D9D6,
            PokemonType::Fighting => 0xC22E28,
            PokemonType::Poison => 0xA33EA1,
            PokemonType::Ground => 0xE2BF65,
            PokemonType::Flying => 0xA98FF3,
            PokemonType::Psychic => 0xF95587,
            PokemonType::Bug => 0xA6B91A,
            PokemonType::Rock => 0xB6A136,
            PokemonType::Ghost => 0x735797,
            PokemonType::Dragon => 0x6F35FC,
            PokemonType::Dark => 0x705746,
            PokemonType::Steel => 0xB7B7CE,
            PokemonType::Fairy => 0xD685AD,
        };
        ((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
    }
}

/// 타입과 그 표시 이름.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeTag {
    pub kind: PokemonType,
    pub name: LocalizedText,
}

/// 게임 버전별 도감 설명 한 항목. 만들 때 공백을 정규화한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlavorText {
    pub language: Language,
    pub text: String,
}

impl FlavorText {
    /// 줄바꿈·폼피드·연속 공백을 공백 하나로 바꿔 만든다.
    pub fn new(language: Language, raw: &str) -> FlavorText {
        let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        FlavorText { language, text }
    }
}

/// 도감 번호 하나에 대응하는 포켓몬 종. 인사 화면의 주인공이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Species {
    pub number: PokedexNumber,
    pub slug: String,
    pub name: LocalizedText,
    pub genus: LocalizedText,
    pub types: Vec<TypeTag>,
    pub flavor_texts: Vec<FlavorText>,
    pub sprite_url: Option<String>,
}

impl Species {
    /// 요청 언어의 도감 설명들. 하나도 없으면 영어 설명들을 돌려준다.
    pub fn flavors_for(&self, lang: Language) -> Vec<&FlavorText> {
        let of = |l: Language| -> Vec<&FlavorText> {
            self.flavor_texts
                .iter()
                .filter(|f| f.language == l)
                .collect()
        };
        let found = of(lang);
        if found.is_empty() && lang != Language::En {
            of(Language::En)
        } else {
            found
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pokedex_label_pads_to_three() {
        assert_eq!(PokedexNumber(25).label(), "No.025");
        assert_eq!(PokedexNumber(1000).label(), "No.1000");
    }

    #[test]
    fn flavor_text_normalizes_whitespace() {
        let f = FlavorText::new(
            Language::Ko,
            "꼬리를 세우고 주변의\n상황을\u{c}살피다  보면",
        );
        assert_eq!(f.text, "꼬리를 세우고 주변의 상황을 살피다 보면");
    }

    #[test]
    fn flavors_for_falls_back_to_english_entries() {
        let mut s = sample();
        s.flavor_texts = vec![
            FlavorText::new(Language::En, "one"),
            FlavorText::new(Language::En, "two"),
        ];
        let got: Vec<&str> = s
            .flavors_for(Language::Ko)
            .iter()
            .map(|f| f.text.as_str())
            .collect();
        assert_eq!(got, vec!["one", "two"]);
    }

    #[test]
    fn flavors_for_prefers_requested_language() {
        let mut s = sample();
        s.flavor_texts = vec![
            FlavorText::new(Language::En, "en"),
            FlavorText::new(Language::Ko, "한"),
        ];
        let got: Vec<&str> = s
            .flavors_for(Language::Ko)
            .iter()
            .map(|f| f.text.as_str())
            .collect();
        assert_eq!(got, vec!["한"]);
    }

    #[test]
    fn type_from_api_name() {
        assert_eq!(
            PokemonType::from_api_name("electric"),
            Some(PokemonType::Electric)
        );
        assert_eq!(PokemonType::from_api_name("stellar"), None);
        assert_eq!(PokemonType::Electric.badge_rgb(), (0xF7, 0xD0, 0x2C));
    }

    #[test]
    fn all_types_roundtrip_api_name() {
        assert_eq!(PokemonType::ALL.len(), 18);
        for t in PokemonType::ALL {
            assert_eq!(PokemonType::from_api_name(t.api_name()), Some(t));
        }
    }

    fn sample() -> Species {
        Species {
            number: PokedexNumber(9001),
            slug: "testmon".into(),
            name: LocalizedText {
                ko: Some("테스트몬".into()),
                en: Some("Testmon".into()),
            },
            genus: LocalizedText { ko: None, en: None },
            types: vec![],
            flavor_texts: vec![],
            sprite_url: None,
        }
    }
}
