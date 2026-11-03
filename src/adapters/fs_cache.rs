//! 파일시스템 캐시.
//!
//! 레이아웃:
//! - `<root>/index.json`: `{"version":1,"entries":[{"id":1,"slug":"..."}]}`
//! - `<root>/species/<id>.json`: 직렬화한 `Species`
//! - `<root>/sprites/<id>.png`: 원본 스프라이트
//!
//! 셸 시작 경로는 작은 인덱스 하나와 종 파일 하나만 읽는다.
//! 모든 쓰기는 같은 디렉터리의 `<이름>.tmp`에 쓴 뒤 `rename`한다(원자적 쓰기).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::ports::{CacheWriter, CatalogError, IndexEntry, SpeciesCatalog, SpriteStore};
use crate::domain::species::Species;

/// 캐시 형식 버전. 형식이 바뀌면 올린다.
pub const CACHE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    entries: Vec<IndexEntry>,
}

/// 파일시스템 캐시 어댑터.
#[derive(Debug, Clone)]
pub struct FsCache {
    root: PathBuf,
}

fn io_err(e: std::io::Error) -> CatalogError {
    CatalogError::Io(e.to_string())
}

impl FsCache {
    pub fn new(root: impl Into<PathBuf>) -> FsCache {
        FsCache { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn species_path(&self, id: u32) -> PathBuf {
        self.root.join("species").join(format!("{id}.json"))
    }

    fn sprite_path(&self, id: u32) -> PathBuf {
        self.root.join("sprites").join(format!("{id}.png"))
    }

    /// 임시 파일에 쓰고 이름을 바꿔, 중간에 끊겨도 반쯤 쓰인 파일이 남지 않게 한다.
    fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CatalogError> {
        let dir = path.parent().expect("캐시 파일에는 상위 디렉터리가 있다");
        fs::create_dir_all(dir).map_err(io_err)?;
        let mut tmp_name = path.file_name().expect("파일 이름").to_os_string();
        tmp_name.push(".tmp");
        let tmp = dir.join(tmp_name);
        fs::write(&tmp, bytes).map_err(io_err)?;
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            io_err(e)
        })
    }
}

impl SpeciesCatalog for FsCache {
    fn index(&self) -> Result<Vec<IndexEntry>, CatalogError> {
        let text = match fs::read_to_string(self.index_path()) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(CatalogError::Empty),
            Err(e) => return Err(io_err(e)),
        };
        // 버전을 먼저 확인해, 형식이 바뀐 인덱스를 "깨짐"이 아니라 "버전 불일치"로 알린다.
        #[derive(Deserialize)]
        struct VersionOnly {
            version: u32,
        }
        let v: VersionOnly =
            serde_json::from_str(&text).map_err(|e| CatalogError::Corrupt(e.to_string()))?;
        if v.version != CACHE_VERSION {
            return Err(CatalogError::VersionMismatch);
        }
        let index: IndexFile =
            serde_json::from_str(&text).map_err(|e| CatalogError::Corrupt(e.to_string()))?;
        if index.entries.is_empty() {
            return Err(CatalogError::Empty);
        }
        Ok(index.entries)
    }

    fn species(&self, id: u32) -> Result<Species, CatalogError> {
        let bytes = fs::read(self.species_path(id)).map_err(io_err)?;
        serde_json::from_slice(&bytes).map_err(|e| CatalogError::Corrupt(e.to_string()))
    }
}

impl SpriteStore for FsCache {
    fn sprite_png(&self, id: u32) -> Option<Vec<u8>> {
        fs::read(self.sprite_path(id)).ok()
    }

    fn has_sprite(&self, id: u32) -> bool {
        self.sprite_path(id).is_file()
    }
}

impl CacheWriter for FsCache {
    fn has_species(&self, id: u32) -> bool {
        self.species_path(id).is_file()
    }

    fn write_species(&self, s: &Species) -> Result<(), CatalogError> {
        let bytes = serde_json::to_vec(s).map_err(|e| CatalogError::Corrupt(e.to_string()))?;
        Self::write_atomic(&self.species_path(s.number.0), &bytes)
    }

    fn write_sprite(&self, id: u32, png: &[u8]) -> Result<(), CatalogError> {
        Self::write_atomic(&self.sprite_path(id), png)
    }

    fn write_index(&self, entries: &[IndexEntry]) -> Result<(), CatalogError> {
        let file = IndexFile {
            version: CACHE_VERSION,
            entries: entries.to_vec(),
        };
        let bytes = serde_json::to_vec(&file).map_err(|e| CatalogError::Corrupt(e.to_string()))?;
        Self::write_atomic(&self.index_path(), &bytes)
    }

    fn cached_index_entries(&self) -> Vec<IndexEntry> {
        let Ok(dir) = fs::read_dir(self.root.join("species")) else {
            return Vec::new();
        };
        let mut entries: Vec<IndexEntry> = dir
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let id: u32 = name.strip_suffix(".json")?.parse().ok()?;
                let s = self.species(id).ok()?;
                Some(IndexEntry { id, slug: s.slug })
            })
            .collect();
        entries.sort_by_key(|e| e.id);
        entries
    }

    fn missing_sprite_url(&self, id: u32) -> Option<String> {
        if self.has_sprite(id) {
            return None;
        }
        self.species(id).ok()?.sprite_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language::LocalizedText;
    use crate::domain::species::PokedexNumber;

    fn species(id: u32, url: Option<&str>) -> Species {
        Species {
            number: PokedexNumber(id),
            slug: format!("testmon-{id}"),
            name: LocalizedText {
                ko: Some("테스트몬".into()),
                en: Some("Testmon".into()),
            },
            genus: LocalizedText::default(),
            types: vec![],
            flavor_texts: vec![],
            sprite_url: url.map(Into::into),
        }
    }

    fn no_tmp_files(dir: &Path) -> bool {
        fs::read_dir(dir).map_or(true, |rd| {
            rd.flatten()
                .all(|e| !e.file_name().to_string_lossy().ends_with(".tmp"))
        })
    }

    #[test]
    fn empty_dir_is_empty_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            FsCache::new(tmp.path()).index(),
            Err(CatalogError::Empty)
        ));
    }

    #[test]
    fn empty_entries_is_empty_error() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = FsCache::new(tmp.path());
        cache.write_index(&[]).unwrap();
        assert!(matches!(cache.index(), Err(CatalogError::Empty)));
    }

    #[test]
    fn species_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = FsCache::new(tmp.path());
        let s = species(25, Some("https://example.invalid/25.png"));
        assert!(!cache.has_species(25));
        cache.write_species(&s).unwrap();
        assert!(cache.has_species(25));
        assert_eq!(cache.species(25).unwrap(), s);
    }

    #[test]
    fn missing_species_is_io_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            FsCache::new(tmp.path()).species(1),
            Err(CatalogError::Io(_))
        ));
    }

    #[test]
    fn index_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = FsCache::new(tmp.path());
        let entries = vec![
            IndexEntry {
                id: 1,
                slug: "a".into(),
            },
            IndexEntry {
                id: 2,
                slug: "b".into(),
            },
        ];
        cache.write_index(&entries).unwrap();
        assert_eq!(cache.index().unwrap(), entries);
    }

    #[test]
    fn version_mismatch_detected() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("index.json"),
            r#"{"version":2,"entries":[{"id":1,"slug":"a"}]}"#,
        )
        .unwrap();
        assert!(matches!(
            FsCache::new(tmp.path()).index(),
            Err(CatalogError::VersionMismatch)
        ));
    }

    #[test]
    fn corrupt_index_is_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("index.json"), "{").unwrap();
        assert!(matches!(
            FsCache::new(tmp.path()).index(),
            Err(CatalogError::Corrupt(_))
        ));
    }

    #[test]
    fn corrupt_species_file_is_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("species")).unwrap();
        fs::write(tmp.path().join("species/5.json"), "{").unwrap();
        assert!(matches!(
            FsCache::new(tmp.path()).species(5),
            Err(CatalogError::Corrupt(_))
        ));
    }

    #[test]
    fn write_is_atomic_no_partial_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = FsCache::new(tmp.path());
        cache.write_species(&species(7, None)).unwrap();
        let mut updated = species(7, None);
        updated.slug = "renamed".into();
        cache.write_species(&updated).unwrap();
        cache.write_sprite(7, b"png").unwrap();
        cache
            .write_index(&[IndexEntry {
                id: 7,
                slug: "renamed".into(),
            }])
            .unwrap();
        assert_eq!(cache.species(7).unwrap().slug, "renamed");
        for dir in [
            tmp.path().to_path_buf(),
            tmp.path().join("species"),
            tmp.path().join("sprites"),
        ] {
            assert!(no_tmp_files(&dir), "{dir:?}에 .tmp가 남음");
        }
    }

    #[test]
    fn cached_index_entries_lists_files() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = FsCache::new(tmp.path());
        for id in [3, 1, 2] {
            cache.write_species(&species(id, None)).unwrap();
        }
        fs::write(tmp.path().join("species/9.json"), "{").unwrap(); // 깨진 파일은 제외
        let entries = cache.cached_index_entries();
        assert_eq!(
            entries.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(entries[0].slug, "testmon-1");
    }

    #[test]
    fn missing_sprite_url_only_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = FsCache::new(tmp.path());
        assert_eq!(cache.missing_sprite_url(1), None); // 종 없음
        cache
            .write_species(&species(1, Some("https://example.invalid/1.png")))
            .unwrap();
        assert_eq!(
            cache.missing_sprite_url(1).as_deref(),
            Some("https://example.invalid/1.png")
        );
        assert!(!cache.has_sprite(1));
        cache.write_sprite(1, b"png").unwrap();
        assert!(cache.has_sprite(1));
        assert_eq!(cache.sprite_png(1).as_deref(), Some(&b"png"[..]));
        assert_eq!(cache.missing_sprite_url(1), None);
        cache.write_species(&species(2, None)).unwrap();
        assert_eq!(cache.missing_sprite_url(2), None);
    }
}
