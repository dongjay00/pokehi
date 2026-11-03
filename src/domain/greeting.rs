//! 인사 화면 값 조립. 화면에 그릴 모든 값을 여기서 확정한다.

use super::language::{Language, LocalizedText};
use super::selection::{pick_index, SelectionMode, FLAVOR_SALT};
use super::species::Species;
use super::sprite::{Sprite, MAX_SPRITE_PX};

/// 타입 뱃지 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badge {
    pub label: String,
    pub rgb: (u8, u8, u8),
}

/// 화면에 그릴 값 묶음.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Greeting {
    pub number_label: String,
    pub name: String,
    /// 한국어 화면에서 덧붙이는 영어 이름.
    pub secondary_name: Option<String>,
    pub badges: Vec<Badge>,
    pub genus: Option<String>,
    pub flavor: Option<String>,
    /// 크롭·축소를 마친 그림. 그림이 없거나 비면 `None`.
    pub sprite: Option<Sprite>,
}

/// 종과 언어, 선택 방식으로 인사 화면 값을 만든다.
pub fn compose(
    species: &Species,
    lang: Language,
    mode: &SelectionMode,
    rng: &mut dyn FnMut() -> u64,
    sprite: Option<Sprite>,
) -> Greeting {
    let name = species.name.get(lang).unwrap_or(&species.slug).to_string();

    let secondary_name = match (lang, species.name.en.as_deref()) {
        (Language::Ko, Some(en)) if en != name => Some(en.to_string()),
        _ => None,
    };

    let badges = species
        .types
        .iter()
        .map(|t| Badge {
            label: label_or(&t.name, lang, t.kind.api_name()),
            rgb: t.kind.badge_rgb(),
        })
        .collect();

    let flavors = species.flavors_for(lang);
    let flavor = pick_index(mode, flavors.len(), FLAVOR_SALT, rng).map(|i| flavors[i].text.clone());

    let sprite = sprite
        .map(|s| {
            s.crop_transparent()
                .fit_within(MAX_SPRITE_PX, MAX_SPRITE_PX)
        })
        .filter(|s| !s.is_empty());

    Greeting {
        number_label: species.number.label(),
        name,
        secondary_name,
        badges,
        genus: species.genus.get(lang).map(str::to_string),
        flavor,
        sprite,
    }
}

fn label_or(text: &LocalizedText, lang: Language, fallback: &str) -> String {
    text.get(lang).unwrap_or(fallback).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::species::{FlavorText, PokedexNumber, PokemonType, TypeTag};
    use chrono::NaiveDate;

    fn text(ko: Option<&str>, en: Option<&str>) -> LocalizedText {
        LocalizedText {
            ko: ko.map(Into::into),
            en: en.map(Into::into),
        }
    }

    // 합성 데이터: 실제 포켓몬 이름·설명이 아니다.
    fn sample_species() -> Species {
        Species {
            number: PokedexNumber(25),
            slug: "testmon".into(),
            name: text(Some("테스트몬"), Some("Testmon")),
            genus: text(Some("시험포켓몬"), Some("Test Pokémon")),
            types: vec![
                TypeTag {
                    kind: PokemonType::Electric,
                    name: text(Some("전기"), Some("Electric")),
                },
                TypeTag {
                    kind: PokemonType::Flying,
                    name: text(None, None),
                },
            ],
            flavor_texts: vec![
                FlavorText::new(Language::Ko, "첫 번째 설명"),
                FlavorText::new(Language::Ko, "두 번째 설명"),
                FlavorText::new(Language::En, "english one"),
            ],
            sprite_url: None,
        }
    }

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Sprite {
        Sprite::new(w, h, vec![px; (w * h) as usize])
    }

    #[test]
    fn korean_greeting_has_english_secondary() {
        let g = compose(
            &sample_species(),
            Language::Ko,
            &SelectionMode::Random,
            &mut || 0,
            None,
        );
        assert_eq!(g.number_label, "No.025");
        assert_eq!(g.name, "테스트몬");
        assert_eq!(g.secondary_name.as_deref(), Some("Testmon"));
        assert_eq!(g.badges[0].label, "전기");
        assert_eq!(g.badges[0].rgb, PokemonType::Electric.badge_rgb());
        assert_eq!(g.genus.as_deref(), Some("시험포켓몬"));
        assert_eq!(g.flavor.as_deref(), Some("첫 번째 설명"));
    }

    #[test]
    fn english_greeting_has_no_secondary() {
        let g = compose(
            &sample_species(),
            Language::En,
            &SelectionMode::Random,
            &mut || 0,
            None,
        );
        assert_eq!(g.name, "Testmon");
        assert_eq!(g.secondary_name, None);
        assert_eq!(g.badges[0].label, "Electric");
        assert_eq!(g.flavor.as_deref(), Some("english one"));
    }

    #[test]
    fn badge_without_names_uses_api_name() {
        let g = compose(
            &sample_species(),
            Language::Ko,
            &SelectionMode::Random,
            &mut || 0,
            None,
        );
        assert_eq!(g.badges[1].label, "flying");
    }

    #[test]
    fn no_secondary_when_names_equal() {
        let mut s = sample_species();
        s.name = text(None, Some("Testmon"));
        let g = compose(&s, Language::Ko, &SelectionMode::Random, &mut || 0, None);
        assert_eq!(g.name, "Testmon");
        assert_eq!(g.secondary_name, None);
    }

    #[test]
    fn name_falls_back_to_slug() {
        let mut s = sample_species();
        s.name = text(None, None);
        let g = compose(&s, Language::Ko, &SelectionMode::Random, &mut || 0, None);
        assert_eq!(g.name, "testmon");
    }

    #[test]
    fn korean_flavor_falls_back_to_english() {
        let mut s = sample_species();
        s.flavor_texts.retain(|f| f.language == Language::En);
        let g = compose(&s, Language::Ko, &SelectionMode::Random, &mut || 0, None);
        assert_eq!(g.flavor.as_deref(), Some("english one"));
    }

    #[test]
    fn flavor_none_when_no_entries() {
        let mut s = sample_species();
        s.flavor_texts.clear();
        let g = compose(&s, Language::Ko, &SelectionMode::Random, &mut || 0, None);
        assert_eq!(g.flavor, None);
    }

    #[test]
    fn random_flavor_uses_rng() {
        let g = compose(
            &sample_species(),
            Language::Ko,
            &SelectionMode::Random,
            &mut || 1,
            None,
        );
        assert_eq!(g.flavor.as_deref(), Some("두 번째 설명"));
    }

    #[test]
    fn daily_flavor_is_stable() {
        let mode = SelectionMode::Daily(NaiveDate::from_ymd_opt(2026, 11, 3).unwrap());
        let a = compose(
            &sample_species(),
            Language::Ko,
            &mode,
            &mut || unreachable!(),
            None,
        );
        let b = compose(
            &sample_species(),
            Language::Ko,
            &mode,
            &mut || unreachable!(),
            None,
        );
        assert!(a.flavor.is_some());
        assert_eq!(a.flavor, b.flavor);
    }

    #[test]
    fn empty_sprite_becomes_none() {
        let sprite = solid(96, 96, [0, 0, 0, 0]);
        let g = compose(
            &sample_species(),
            Language::Ko,
            &SelectionMode::Random,
            &mut || 0,
            Some(sprite),
        );
        assert_eq!(g.sprite, None);
    }

    #[test]
    fn sprite_is_cropped_and_fitted() {
        let mut pixels = vec![[0, 0, 0, 0]; 96 * 96];
        for y in 16..80 {
            for x in 16..80 {
                pixels[y * 96 + x] = [1, 2, 3, 255];
            }
        }
        let sprite = Sprite::new(96, 96, pixels);
        let g = compose(
            &sample_species(),
            Language::Ko,
            &SelectionMode::Random,
            &mut || 0,
            Some(sprite),
        );
        let s = g.sprite.unwrap();
        assert_eq!((s.width, s.height), (32, 32));
    }
}
