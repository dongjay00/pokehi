//! CLI 단위 테스트. 가짜 Context와 임시 디렉터리 캐시를 쓴다. 네트워크는 쓰지 않는다.

use super::*;
use crate::adapters::fs_cache::FsCache;
use crate::app::ports::{CacheWriter, IndexEntry, SourceError};
use crate::domain::language::LocalizedText;
use crate::domain::species::{FlavorText, PokedexNumber, PokemonType, Species, TypeTag};
use std::path::Path;

struct FailingSource;

impl PokedexSource for FailingSource {
    fn species_ids(&self) -> Result<Vec<u32>, SourceError> {
        Err(SourceError::Permanent("목록 실패".into()))
    }
    fn type_names(&self, _: &str) -> Result<LocalizedText, SourceError> {
        unreachable!()
    }
    fn species(&self, _: u32, _: &HashMap<String, LocalizedText>) -> Result<Species, SourceError> {
        unreachable!()
    }
    fn sprite(&self, _: &str) -> Result<Vec<u8>, SourceError> {
        unreachable!()
    }
}

/// 종마다 실패하는 소스: 목록은 받지만 한 종도 받지 못한다.
struct NothingSource;

impl PokedexSource for NothingSource {
    fn species_ids(&self) -> Result<Vec<u32>, SourceError> {
        Ok(vec![1, 2])
    }
    fn type_names(&self, _: &str) -> Result<LocalizedText, SourceError> {
        Ok(LocalizedText::default())
    }
    fn species(&self, _: u32, _: &HashMap<String, LocalizedText>) -> Result<Species, SourceError> {
        Err(SourceError::Permanent("404".into()))
    }
    fn sprite(&self, _: &str) -> Result<Vec<u8>, SourceError> {
        unreachable!()
    }
}

fn png_2x2() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, 2, 2);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let data = [255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 255, 0, 255, 0, 255];
        enc.write_header().unwrap().write_image_data(&data).unwrap();
    }
    out
}

// 합성 데이터: 실제 포켓몬이 아니다.
fn seed(dir: &Path) {
    let cache = FsCache::new(dir);
    let mk = |id: u32, slug: &str, ko: &str, en: &str| Species {
        number: PokedexNumber(id),
        slug: slug.into(),
        name: LocalizedText {
            ko: Some(ko.into()),
            en: Some(en.into()),
        },
        genus: LocalizedText {
            ko: Some("시험포켓몬".into()),
            en: Some("Test Pokémon".into()),
        },
        types: vec![TypeTag {
            kind: PokemonType::Electric,
            name: LocalizedText {
                ko: Some("전기".into()),
                en: Some("Electric".into()),
            },
        }],
        flavor_texts: vec![
            FlavorText::new(Language::Ko, "합성 설명 문장"),
            FlavorText::new(Language::En, "A synthetic entry"),
        ],
        sprite_url: None,
    };
    let list = [
        mk(9001, "testmon", "테스트몬", "Testmon"),
        mk(9002, "othermon", "다른몬", "Othermon"),
    ];
    for s in &list {
        cache.write_species(s).unwrap();
        cache.write_sprite(s.number.0, &png_2x2()).unwrap();
    }
    cache
        .write_index(
            &list
                .iter()
                .map(|s| IndexEntry {
                    id: s.number.0,
                    slug: s.slug.clone(),
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
}

struct Run {
    code: i32,
    out: String,
    err: String,
}

struct Setup<'a> {
    args: &'a [&'a str],
    env: Vec<(&'a str, String)>,
    tty: bool,
    source: fn() -> Box<dyn PokedexSource>,
}

fn setup<'a>(dir: &Path, args: &'a [&'a str]) -> Setup<'a> {
    Setup {
        args,
        env: vec![("POKEHI_DATA_DIR", dir.to_string_lossy().into_owned())],
        tty: true,
        source: || Box::new(FailingSource),
    }
}

impl Setup<'_> {
    fn env(mut self, k: &'static str, v: &str) -> Self {
        self.env.push((k, v.to_string()));
        self
    }
    fn tty(mut self, tty: bool) -> Self {
        self.tty = tty;
        self
    }
    fn run(self) -> Run {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = {
            let mut ctx = Context {
                env: self
                    .env
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                stdout_is_tty: self.tty,
                term_width: Some(100),
                windows_vt_ok: None,
                today: NaiveDate::from_ymd_opt(2026, 11, 3).unwrap(),
                out: &mut out,
                err: &mut err,
                rng: Box::new(|| 0),
                make_source: Box::new(self.source),
            };
            let mut args = vec![OsString::from("pokehi")];
            args.extend(self.args.iter().map(OsString::from));
            run(args, &mut ctx)
        };
        Run {
            code,
            out: String::from_utf8(out).unwrap(),
            err: String::from_utf8(err).unwrap(),
        }
    }
}

fn seeded() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    seed(tmp.path());
    tmp
}

#[test]
fn greets_from_cache() {
    let tmp = seeded();
    let r = setup(tmp.path(), &[]).run();
    assert_eq!(r.code, 0);
    assert!(r.out.contains("No.9001"), "{}", r.out);
    assert!(r.out.contains("테스트몬"));
    assert!(r.out.contains('▀'));
    assert!(r.err.is_empty());
}

#[test]
fn greets_from_cache_with_force() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--force"]).tty(false).run();
    assert_eq!(r.code, 0);
    assert!(r.out.contains("No.9001"));
}

#[test]
fn non_tty_prints_nothing() {
    let tmp = seeded();
    let r = setup(tmp.path(), &[]).tty(false).run();
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (0, "", ""));
    let r = setup(tmp.path(), &["--tody"])
        .tty(false)
        .env("POKEHI_LANG", "fr")
        .run();
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (0, "", ""));
}

#[test]
fn no_cache_hint_exit_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let r = setup(tmp.path(), &[]).run();
    assert_eq!(r.code, 0);
    assert_eq!(r.err, format!("{}\n", message(Language::Ko, Msg::NoCache)));
    assert!(r.out.is_empty());
}

#[test]
fn version_mismatch_hint_exit_zero() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("index.json"),
        r#"{"version":99,"entries":[]}"#,
    )
    .unwrap();
    let r = setup(tmp.path(), &[]).run();
    assert_eq!(r.code, 0);
    assert_eq!(
        r.err,
        format!("{}\n", message(Language::Ko, Msg::VersionMismatch))
    );
}

#[test]
fn bad_option_in_greeting_mode_exit_zero() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--tody"]).run();
    assert_eq!(r.code, 0);
    assert_eq!(r.err, format!("{}\n", message(Language::Ko, Msg::BadArgs)));
    assert!(r.out.is_empty());
}

#[test]
fn bad_option_in_sync_exit_two() {
    let tmp = tempfile::tempdir().unwrap();
    let r = setup(tmp.path(), &["sync", "--x"]).run();
    assert_eq!(r.code, 2);
    assert!(!r.err.is_empty());
}

#[test]
fn help_goes_to_stdout_exit_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let r = setup(tmp.path(), &["--help"]).tty(false).run();
    assert_eq!(r.code, 0);
    assert!(r.out.contains("sync"));
    let r = setup(tmp.path(), &["--version"]).run();
    assert_eq!(r.code, 0);
    assert!(r.out.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn lang_flag_beats_env() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--lang", "en", "--pokemon", "9001"])
        .env("POKEHI_LANG", "ko")
        .run();
    assert!(r.out.contains("Testmon"));
    assert!(!r.out.contains("테스트몬"));
    assert!(r.out.contains("Electric"));
}

#[test]
fn lang_env_used_without_flag() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--pokemon", "9001"])
        .env("POKEHI_LANG", "EN")
        .run();
    assert!(r.out.contains("Testmon"));
    assert!(!r.out.contains("테스트몬"));
}

#[test]
fn bad_lang_env_warns_and_uses_ko() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--pokemon", "9001"])
        .env("POKEHI_LANG", "fr")
        .run();
    assert_eq!(r.code, 0);
    assert_eq!(
        r.err,
        format!("{}\n", message(Language::Ko, Msg::BadLangEnv("fr".into())))
    );
    assert!(r.out.contains("테스트몬"));
}

#[test]
fn no_color_env_strips_escapes() {
    let tmp = seeded();
    let r = setup(tmp.path(), &[]).env("NO_COLOR", "1").run();
    assert!(!r.out.contains('\x1b'));
    assert!(r.out.contains("[전기]"));
}

#[test]
fn no_color_beats_forced_color() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--color", "truecolor"])
        .env("NO_COLOR", "1")
        .run();
    assert!(!r.out.contains('\x1b'));
}

#[test]
fn color_flag_forces_256() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--color", "256"])
        .env("COLORTERM", "truecolor")
        .run();
    assert!(r.out.contains("\x1b[38;5;"));
    assert!(!r.out.contains("\x1b[38;2;"));
}

#[test]
fn auto_color_detects_truecolor() {
    let tmp = seeded();
    let r = setup(tmp.path(), &[]).env("COLORTERM", "truecolor").run();
    assert!(r.out.contains("\x1b[38;2;"));
}

#[test]
fn today_is_stable() {
    let tmp = seeded();
    let a = setup(tmp.path(), &["--today"]).run();
    let b = setup(tmp.path(), &["--today"]).run();
    assert_eq!(a.code, 0);
    assert!(a.out.contains("No.900"));
    assert_eq!(a.out, b.out);
}

#[test]
fn pokemon_by_slug() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--pokemon", "OtherMon"]).run();
    assert_eq!(r.code, 0);
    assert!(r.out.contains("다른몬"));
}

#[test]
fn pokemon_not_found_exit_one() {
    let tmp = seeded();
    let r = setup(tmp.path(), &["--pokemon", "9999"]).run();
    assert_eq!(r.code, 1);
    assert_eq!(
        r.err,
        format!("{}\n", message(Language::Ko, Msg::NotFound("9999".into())))
    );
}

#[test]
fn english_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let r = setup(tmp.path(), &["--lang", "en"]).run();
    assert_eq!(r.err, format!("{}\n", message(Language::En, Msg::NoCache)));
}

#[test]
fn sync_list_failure_exit_one() {
    let tmp = tempfile::tempdir().unwrap();
    let r = setup(tmp.path(), &["sync"]).run();
    assert_eq!(r.code, 1);
    assert!(
        r.err.contains("pokehi: 동기화 실패: 목록 실패"),
        "{}",
        r.err
    );
}

#[test]
fn sync_nothing_saved_exit_one() {
    let tmp = tempfile::tempdir().unwrap();
    let mut s = setup(tmp.path(), &["sync"]);
    s.source = || Box::new(NothingSource);
    let r = s.run();
    assert_eq!(r.code, 1);
    assert!(r.err.contains("동기화 중 2/2"));
    assert!(r.err.contains(&message(
        Language::Ko,
        Msg::SyncFailed(message(Language::Ko, Msg::NothingFetched))
    )));
    assert!(r.out.contains("완료: 0종 저장, 실패 2건"));
}

#[test]
fn sync_lang_flag_after_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let r = setup(tmp.path(), &["sync", "--lang", "en"]).run();
    assert_eq!(r.code, 1);
    assert!(r.err.contains("pokehi: sync failed:"));
}
