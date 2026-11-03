//! 포트: 유스케이스가 바깥 세계에 요구하는 기능을 trait로 정의한다.

use serde::{Deserialize, Serialize};

use crate::domain::species::Species;

/// 로컬 캐시를 읽거나 쓸 때의 오류.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// 캐시가 없거나 비어 있다.
    Empty,
    /// 캐시 형식 버전이 다르다.
    VersionMismatch,
    /// 파일 내용이 깨졌다.
    Corrupt(String),
    /// 파일을 읽거나 쓸 수 없다.
    Io(String),
}

/// 인덱스 한 항목. 셸 시작 경로에서는 이 작은 목록만 파싱한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: u32,
    pub slug: String,
}

/// 캐시된 종 데이터 읽기.
pub trait SpeciesCatalog {
    fn index(&self) -> Result<Vec<IndexEntry>, CatalogError>;
    fn species(&self, id: u32) -> Result<Species, CatalogError>;
}

/// 캐시된 스프라이트 PNG 읽기.
pub trait SpriteStore {
    fn sprite_png(&self, id: u32) -> Option<Vec<u8>>;
    fn has_sprite(&self, id: u32) -> bool;
}

/// `sync`가 캐시를 채울 때 쓰는 쓰기 포트. 여러 스레드에서 함께 쓴다.
pub trait CacheWriter: Send + Sync {
    fn has_species(&self, id: u32) -> bool;
    fn write_species(&self, s: &Species) -> Result<(), CatalogError>;
    fn write_sprite(&self, id: u32, png: &[u8]) -> Result<(), CatalogError>;
    fn write_index(&self, entries: &[IndexEntry]) -> Result<(), CatalogError>;
    /// 읽을 수 있는 종 파일들의 (id, slug)를 id 오름차순으로.
    fn cached_index_entries(&self) -> Vec<IndexEntry>;
    /// 종이 캐시에 있고 `sprite_url`이 있는데 스프라이트 파일이 없을 때만 그 URL.
    fn missing_sprite_url(&self, id: u32) -> Option<String>;
}
