//! ShowGreeting 유스케이스: 캐시에서 종을 골라 인사 화면 값을 만든다.
//! 네트워크를 쓰지 않는다.

use crate::adapters::png_decode::decode_png;
use crate::app::ports::{CatalogError, IndexEntry, SpeciesCatalog, SpriteStore};
use crate::domain::greeting::{compose, Greeting};
use crate::domain::language::Language;
use crate::domain::selection::{pick_index, SelectionMode};
use crate::domain::species::Species;

/// 종 데이터가 깨졌을 때 다른 종으로 다시 고르는 최대 횟수.
const MAX_ATTEMPTS: u64 = 3;

/// 어떤 포켓몬을 보여 줄지.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// 선택 규칙으로 고른다.
    Pick(SelectionMode),
    /// 지정한 포켓몬. 숫자면 도감 번호, 아니면 영문 slug(대소문자 무시).
    Specific(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GreetingError {
    NoCache,
    VersionMismatch,
    NotFound(String),
    Corrupt,
}

fn find_entry<'a>(entries: &'a [IndexEntry], query: &str) -> Option<&'a IndexEntry> {
    let query = query.trim();
    match query.parse::<u32>() {
        Ok(id) => entries.iter().find(|e| e.id == id),
        Err(_) => {
            let slug = query.to_lowercase();
            entries.iter().find(|e| e.slug.to_lowercase() == slug)
        }
    }
}

/// 인사 화면 값을 만든다. 스프라이트가 없거나 깨졌으면 그림 없이 만든다.
pub fn show_greeting(
    catalog: &dyn SpeciesCatalog,
    sprites: &dyn SpriteStore,
    target: &Target,
    lang: Language,
    rng: &mut dyn FnMut() -> u64,
) -> Result<Greeting, GreetingError> {
    let entries = catalog.index().map_err(|e| match e {
        CatalogError::Empty => GreetingError::NoCache,
        CatalogError::VersionMismatch => GreetingError::VersionMismatch,
        CatalogError::Corrupt(_) | CatalogError::Io(_) => GreetingError::Corrupt,
    })?;

    let (species, mode): (Species, SelectionMode) = match target {
        Target::Specific(query) => {
            let not_found = || GreetingError::NotFound(query.clone());
            let entry = find_entry(&entries, query).ok_or_else(not_found)?;
            let species = catalog.species(entry.id).map_err(|_| not_found())?;
            (species, SelectionMode::Random)
        }
        Target::Pick(mode) => {
            let found = (0..MAX_ATTEMPTS).find_map(|attempt| {
                let i = pick_index(mode, entries.len(), attempt, rng)?;
                catalog.species(entries[i].id).ok()
            });
            (found.ok_or(GreetingError::Corrupt)?, *mode)
        }
    };

    let sprite = sprites
        .sprite_png(species.number.0)
        .and_then(|bytes| decode_png(&bytes));
    Ok(compose(&species, lang, &mode, rng, sprite))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::IndexEntry;
    use crate::domain::language::LocalizedText;
    use crate::domain::species::{PokedexNumber, Species};
    use chrono::NaiveDate;
    use std::collections::HashMap;

    struct FakeCatalog {
        index: Result<Vec<IndexEntry>, CatalogError>,
        species: HashMap<u32, Result<Species, CatalogError>>,
    }

    impl SpeciesCatalog for FakeCatalog {
        fn index(&self) -> Result<Vec<IndexEntry>, CatalogError> {
            self.index.clone()
        }
        fn species(&self, id: u32) -> Result<Species, CatalogError> {
            self.species
                .get(&id)
                .cloned()
                .unwrap_or(Err(CatalogError::Io("없음".into())))
        }
    }

    #[derive(Default)]
    struct FakeSprites(HashMap<u32, Vec<u8>>);

    impl SpriteStore for FakeSprites {
        fn sprite_png(&self, id: u32) -> Option<Vec<u8>> {
            self.0.get(&id).cloned()
        }
        fn has_sprite(&self, id: u32) -> bool {
            self.0.contains_key(&id)
        }
    }

    // 합성 데이터.
    fn species(id: u32, slug: &str, en: &str) -> Species {
        Species {
            number: PokedexNumber(id),
            slug: slug.into(),
            name: LocalizedText {
                ko: None,
                en: Some(en.into()),
            },
            genus: LocalizedText::default(),
            types: vec![],
            flavor_texts: vec![],
            sprite_url: None,
        }
    }

    fn catalog(list: &[(u32, &str, &str)]) -> FakeCatalog {
        FakeCatalog {
            index: Ok(list
                .iter()
                .map(|&(id, slug, _)| IndexEntry {
                    id,
                    slug: slug.into(),
                })
                .collect()),
            species: list
                .iter()
                .map(|&(id, slug, en)| (id, Ok(species(id, slug, en))))
                .collect(),
        }
    }

    fn three() -> FakeCatalog {
        catalog(&[
            (1, "alpha", "Alpha"),
            (2, "beta", "Beta"),
            (9001, "testmon", "Testmon"),
        ])
    }

    fn pick(c: &FakeCatalog, rng: &mut dyn FnMut() -> u64) -> Result<Greeting, GreetingError> {
        show_greeting(
            c,
            &FakeSprites::default(),
            &Target::Pick(SelectionMode::Random),
            Language::En,
            rng,
        )
    }

    fn specific(c: &FakeCatalog, s: &str) -> Result<Greeting, GreetingError> {
        show_greeting(
            c,
            &FakeSprites::default(),
            &Target::Specific(s.into()),
            Language::En,
            &mut || 0,
        )
    }

    fn tiny_png() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, 1, 1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&[1, 2, 3, 255])
                .unwrap();
        }
        out
    }

    #[test]
    fn picks_from_index() {
        assert_eq!(pick(&three(), &mut || 1).unwrap().name, "Beta");
    }

    #[test]
    fn daily_pick_is_stable() {
        let mode = SelectionMode::Daily(NaiveDate::from_ymd_opt(2026, 11, 3).unwrap());
        let run = || {
            show_greeting(
                &three(),
                &FakeSprites::default(),
                &Target::Pick(mode),
                Language::En,
                &mut || unreachable!(),
            )
            .unwrap()
            .name
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn empty_cache_is_no_cache() {
        let c = FakeCatalog {
            index: Err(CatalogError::Empty),
            species: HashMap::new(),
        };
        assert_eq!(pick(&c, &mut || 0), Err(GreetingError::NoCache));
    }

    #[test]
    fn version_mismatch_maps() {
        let c = FakeCatalog {
            index: Err(CatalogError::VersionMismatch),
            species: HashMap::new(),
        };
        assert_eq!(pick(&c, &mut || 0), Err(GreetingError::VersionMismatch));
    }

    #[test]
    fn corrupt_index_maps_to_corrupt() {
        let c = FakeCatalog {
            index: Err(CatalogError::Corrupt("x".into())),
            species: HashMap::new(),
        };
        assert_eq!(pick(&c, &mut || 0), Err(GreetingError::Corrupt));
    }

    #[test]
    fn corrupt_species_retries_then_hints() {
        let mut c = catalog(&[(1, "alpha", "Alpha"), (2, "beta", "Beta")]);
        c.species
            .insert(1, Err(CatalogError::Corrupt("깨짐".into())));
        let mut seq = [0u64, 1].into_iter();
        assert_eq!(pick(&c, &mut || seq.next().unwrap()).unwrap().name, "Beta");

        c.species
            .insert(2, Err(CatalogError::Corrupt("깨짐".into())));
        let mut calls = 0;
        let result = pick(&c, &mut || {
            calls += 1;
            calls
        });
        assert_eq!(result, Err(GreetingError::Corrupt));
        assert_eq!(calls, 3, "최대 3회까지만 시도한다");
    }

    #[test]
    fn specific_by_number_and_slug() {
        assert_eq!(specific(&three(), "9001").unwrap().name, "Testmon");
        assert_eq!(specific(&three(), "TESTMON").unwrap().name, "Testmon");
        assert_eq!(specific(&three(), "009001").unwrap().name, "Testmon");
    }

    #[test]
    fn specific_missing_is_not_found() {
        assert_eq!(
            specific(&three(), "9999"),
            Err(GreetingError::NotFound("9999".into()))
        );
        assert_eq!(
            specific(&three(), "nothing"),
            Err(GreetingError::NotFound("nothing".into()))
        );
    }

    #[test]
    fn specific_broken_species_is_not_found() {
        let mut c = three();
        c.species
            .insert(1, Err(CatalogError::Corrupt("깨짐".into())));
        assert_eq!(specific(&c, "1"), Err(GreetingError::NotFound("1".into())));
    }

    #[test]
    fn missing_sprite_still_greets() {
        assert_eq!(pick(&three(), &mut || 0).unwrap().sprite, None);
    }

    #[test]
    fn broken_sprite_still_greets() {
        let sprites = FakeSprites([(1, b"not png".to_vec())].into());
        let g = show_greeting(
            &three(),
            &sprites,
            &Target::Pick(SelectionMode::Random),
            Language::En,
            &mut || 0,
        )
        .unwrap();
        assert_eq!(g.sprite, None);
    }

    #[test]
    fn sprite_is_decoded() {
        let sprites = FakeSprites([(1, tiny_png())].into());
        let g = show_greeting(
            &three(),
            &sprites,
            &Target::Pick(SelectionMode::Random),
            Language::En,
            &mut || 0,
        )
        .unwrap();
        assert_eq!(g.sprite.map(|s| (s.width, s.height)), Some((1, 1)));
    }
}
