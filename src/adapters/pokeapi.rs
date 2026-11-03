//! PokéAPI 응답 파싱. 필요한 필드만 뽑아 도메인 타입으로 바꾸는 순수 함수들이다.

use std::collections::HashMap;
use std::fmt;

use serde::Deserialize;

use crate::domain::language::{Language, LocalizedText};
use crate::domain::species::{FlavorText, PokedexNumber, PokemonType, Species, TypeTag};

/// 응답 파싱 실패.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "응답 파싱 실패: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

impl From<serde_json::Error> for ParseError {
    fn from(e: serde_json::Error) -> Self {
        ParseError(e.to_string())
    }
}

/// `pokemon-species/{id}`에서 뽑은 값.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeciesPart {
    pub id: u32,
    pub slug: String,
    pub name: LocalizedText,
    pub genus: LocalizedText,
    pub flavor_texts: Vec<FlavorText>,
    /// 기본 폼(`varieties`의 `is_default`) `pokemon` 리소스 URL.
    pub default_pokemon_url: String,
}

/// `pokemon/{id}`에서 뽑은 값.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokemonPart {
    /// `slot` 오름차순 타입 API 이름.
    pub type_names: Vec<String>,
    pub sprite_url: Option<String>,
}

// ---- API 응답 구조(필요한 필드만) ----

#[derive(Deserialize)]
struct Resource {
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: String,
}

#[derive(Deserialize)]
struct ListResponse {
    results: Vec<Resource>,
}

#[derive(Deserialize)]
struct NameEntry {
    name: String,
    language: Resource,
}

#[derive(Deserialize)]
struct GenusEntry {
    genus: String,
    language: Resource,
}

#[derive(Deserialize)]
struct FlavorEntry {
    flavor_text: String,
    language: Resource,
}

#[derive(Deserialize)]
struct Variety {
    is_default: bool,
    pokemon: Resource,
}

#[derive(Deserialize)]
struct SpeciesResponse {
    id: u32,
    name: String,
    #[serde(default)]
    names: Vec<NameEntry>,
    #[serde(default)]
    genera: Vec<GenusEntry>,
    #[serde(default)]
    flavor_text_entries: Vec<FlavorEntry>,
    #[serde(default)]
    varieties: Vec<Variety>,
}

#[derive(Deserialize)]
struct TypeSlot {
    slot: u32,
    #[serde(rename = "type")]
    kind: Resource,
}

#[derive(Deserialize)]
struct Sprites {
    front_default: Option<String>,
}

#[derive(Deserialize)]
struct PokemonResponse {
    types: Vec<TypeSlot>,
    sprites: Sprites,
}

#[derive(Deserialize)]
struct TypeResponse {
    #[serde(default)]
    names: Vec<NameEntry>,
}

/// `(언어 코드, 값)` 목록에서 ko/en만 골라 담는다. 같은 언어가 여럿이면 첫 값을 쓴다.
fn localized<'a>(entries: impl Iterator<Item = (&'a str, &'a str)>) -> LocalizedText {
    let mut text = LocalizedText::default();
    for (code, value) in entries {
        let slot = match Language::parse(code) {
            Some(Language::Ko) => &mut text.ko,
            Some(Language::En) => &mut text.en,
            None => continue,
        };
        if slot.is_none() {
            *slot = Some(value.to_string());
        }
    }
    text
}

/// URL 끝(`.../9001/`)의 숫자 id.
fn trailing_id(url: &str) -> Option<u32> {
    url.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

/// `pokemon-species?limit=...` 목록에서 id를 오름차순으로 뽑는다.
pub fn parse_species_list(json: &str) -> Result<Vec<u32>, ParseError> {
    let list: ListResponse = serde_json::from_str(json)?;
    let mut ids = list
        .results
        .iter()
        .map(|r| trailing_id(&r.url).ok_or_else(|| ParseError(format!("id 없는 URL: {}", r.url))))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// `pokemon-species/{id}` 응답을 파싱한다. ko/en 항목만 남긴다.
pub fn parse_species(json: &str) -> Result<SpeciesPart, ParseError> {
    let r: SpeciesResponse = serde_json::from_str(json)?;
    let default_pokemon_url = r
        .varieties
        .iter()
        .find(|v| v.is_default)
        .map(|v| v.pokemon.url.clone())
        .ok_or_else(|| ParseError(format!("종 {}에 기본 폼이 없음", r.id)))?;
    let flavor_texts = r
        .flavor_text_entries
        .iter()
        .filter_map(|e| {
            Language::parse(&e.language.name).map(|l| FlavorText::new(l, &e.flavor_text))
        })
        .collect();
    Ok(SpeciesPart {
        id: r.id,
        slug: r.name,
        name: localized(
            r.names
                .iter()
                .map(|n| (n.language.name.as_str(), n.name.as_str())),
        ),
        genus: localized(
            r.genera
                .iter()
                .map(|g| (g.language.name.as_str(), g.genus.as_str())),
        ),
        flavor_texts,
        default_pokemon_url,
    })
}

/// `pokemon/{id}` 응답에서 타입(slot 순)과 기본 앞모습 스프라이트 URL을 뽑는다.
pub fn parse_pokemon(json: &str) -> Result<PokemonPart, ParseError> {
    let mut r: PokemonResponse = serde_json::from_str(json)?;
    r.types.sort_by_key(|t| t.slot);
    Ok(PokemonPart {
        type_names: r.types.into_iter().map(|t| t.kind.name).collect(),
        sprite_url: r.sprites.front_default,
    })
}

/// `type/{name}` 응답에서 ko/en 이름을 뽑는다.
pub fn parse_type_names(json: &str) -> Result<LocalizedText, ParseError> {
    let r: TypeResponse = serde_json::from_str(json)?;
    Ok(localized(
        r.names
            .iter()
            .map(|n| (n.language.name.as_str(), n.name.as_str())),
    ))
}

/// 종·포켓몬 응답과 타입 이름 표로 `Species`를 만든다. 18종에 없는 타입은 버린다.
pub fn assemble(
    species: SpeciesPart,
    pokemon: PokemonPart,
    type_names: &HashMap<String, LocalizedText>,
) -> Species {
    let types = pokemon
        .type_names
        .iter()
        .filter_map(|name| {
            PokemonType::from_api_name(name).map(|kind| TypeTag {
                kind,
                name: type_names.get(name).cloned().unwrap_or_default(),
            })
        })
        .collect();
    Species {
        number: PokedexNumber(species.id),
        slug: species.slug,
        name: species.name,
        genus: species.genus,
        types,
        flavor_texts: species.flavor_texts,
        sprite_url: pokemon.sprite_url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::species::PokemonType;

    const LIST: &str = include_str!("../../tests/fixtures/species-list.json");
    const SPECIES_A: &str = include_str!("../../tests/fixtures/species-a.json");
    const SPECIES_B: &str = include_str!("../../tests/fixtures/species-b.json");
    const POKEMON_A: &str = include_str!("../../tests/fixtures/pokemon-a.json");
    const POKEMON_C: &str = include_str!("../../tests/fixtures/pokemon-c.json");
    const TYPE_ELECTRIC: &str = include_str!("../../tests/fixtures/type-electric.json");

    fn type_names() -> HashMap<String, LocalizedText> {
        let mut m = HashMap::new();
        m.insert(
            "electric".to_string(),
            parse_type_names(TYPE_ELECTRIC).unwrap(),
        );
        m
    }

    #[test]
    fn species_list_ids_sorted() {
        assert_eq!(parse_species_list(LIST).unwrap(), vec![9001, 9002, 9003]);
    }

    #[test]
    fn species_extracts_ko_en_only() {
        let s = parse_species(SPECIES_A).unwrap();
        assert_eq!(s.id, 9001);
        assert_eq!(s.slug, "testmon");
        assert_eq!(s.name.get(Language::Ko), Some("테스트몬"));
        assert_eq!(s.name.get(Language::En), Some("Testmon"));
        assert_eq!(s.genus.get(Language::Ko), Some("시험포켓몬"));
        assert_eq!(s.flavor_texts.len(), 3); // ja 제외
        assert_eq!(s.flavor_texts[0].text, "합성 설명 첫 번째 문장이다.");
        assert!(s.default_pokemon_url.ends_with("/pokemon/9001/"));
    }

    #[test]
    fn species_without_korean_flavor() {
        let s = parse_species(SPECIES_B).unwrap();
        let ko = s
            .flavor_texts
            .iter()
            .filter(|f| f.language == Language::Ko)
            .count();
        let en = s
            .flavor_texts
            .iter()
            .filter(|f| f.language == Language::En)
            .count();
        assert_eq!((ko, en), (0, 1));
    }

    #[test]
    fn pokemon_types_in_slot_order() {
        assert_eq!(
            parse_pokemon(POKEMON_A).unwrap().type_names,
            vec!["electric", "flying"]
        );
    }

    #[test]
    fn pokemon_sprite_url() {
        let url = parse_pokemon(POKEMON_A).unwrap().sprite_url.unwrap();
        assert!(url.ends_with("/9001.png"));
    }

    #[test]
    fn pokemon_null_sprite() {
        assert_eq!(parse_pokemon(POKEMON_C).unwrap().sprite_url, None);
    }

    #[test]
    fn type_names_ko_en() {
        let t = parse_type_names(TYPE_ELECTRIC).unwrap();
        assert_eq!(t.ko.as_deref(), Some("전기"));
        assert_eq!(t.en.as_deref(), Some("Electric"));
    }

    #[test]
    fn assemble_builds_species() {
        let s = assemble(
            parse_species(SPECIES_A).unwrap(),
            parse_pokemon(POKEMON_A).unwrap(),
            &type_names(),
        );
        assert_eq!(s.number.0, 9001);
        assert_eq!(s.types.len(), 2);
        assert_eq!(s.types[0].kind, PokemonType::Electric);
        assert_eq!(s.types[0].name.ko.as_deref(), Some("전기"));
        // 이름을 받지 못한 타입은 빈 이름으로 둔다.
        assert_eq!(s.types[1].kind, PokemonType::Flying);
        assert_eq!(s.types[1].name, LocalizedText::default());
        assert!(s.sprite_url.is_some());
    }

    #[test]
    fn assemble_drops_unknown_types() {
        let s = assemble(
            parse_species(SPECIES_B).unwrap(),
            parse_pokemon(POKEMON_C).unwrap(),
            &type_names(),
        );
        let kinds: Vec<_> = s.types.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![PokemonType::Water]);
        assert_eq!(s.sprite_url, None);
    }

    #[test]
    fn malformed_json_is_error() {
        assert!(parse_species("{").is_err());
        assert!(parse_pokemon("[]").is_err());
        assert!(
            parse_species_list(r#"{"results":[{"name":"x","url":"https://x/abc/"}]}"#).is_err()
        );
    }

    #[test]
    fn species_without_default_variety_is_error() {
        let json = SPECIES_B.replace("\"is_default\": true", "\"is_default\": false");
        assert!(parse_species(&json).is_err());
    }
}
