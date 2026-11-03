//! SyncPokedex 유스케이스: PokéAPI에서 받아 로컬 캐시를 채운다.
//!
//! 이미 받은 종은 다시 요청하지 않고, 빠진 스프라이트만 다시 받는다.
//! 동시 요청은 최대 4개다(PokéAPI 공정 이용).

use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

use crate::app::ports::{CacheWriter, CatalogError, IndexEntry, PokedexSource, SourceError};
use crate::domain::language::LocalizedText;
use crate::domain::species::{PokemonType, Species};

/// 동시 요청 상한.
pub const MAX_WORKERS: usize = 4;

/// 동기화 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SyncReport {
    /// 원격 목록의 종 수.
    pub total: usize,
    /// 이번에 새로 저장한 종 수.
    pub saved: usize,
    /// 이미 캐시에 있던 종 수.
    pub skipped: usize,
    /// 받지 못한 종 수.
    pub failed: usize,
    /// 종은 저장했지만 그림을 받지 못한 수.
    pub sprites_failed: usize,
}

enum Job {
    /// 종 데이터와 스프라이트를 모두 받는다.
    Full(u32),
    /// 종은 이미 있고 스프라이트만 받는다.
    Sprite(u32, String),
}

enum Outcome {
    Saved { sprite_ok: bool },
    Failed,
    SpriteOnly { ok: bool },
}

/// 일시적 실패는 `delays`만큼 기다리며 다시 시도한다. 영구 실패는 바로 돌려준다.
fn with_retry<T>(
    delays: &[Duration],
    mut f: impl FnMut() -> Result<T, SourceError>,
) -> Result<T, SourceError> {
    let mut remaining = delays.iter();
    loop {
        match f() {
            Err(SourceError::Transient(msg)) => match remaining.next() {
                Some(delay) => std::thread::sleep(*delay),
                None => return Err(SourceError::Transient(msg)),
            },
            other => return other,
        }
    }
}

fn fetch_sprite(
    source: &dyn PokedexSource,
    cache: &dyn CacheWriter,
    id: u32,
    url: &str,
    delays: &[Duration],
) -> bool {
    with_retry(delays, || source.sprite(url))
        .ok()
        .is_some_and(|png| cache.write_sprite(id, &png).is_ok())
}

fn run_job(
    job: Job,
    source: &dyn PokedexSource,
    cache: &dyn CacheWriter,
    types: &HashMap<String, LocalizedText>,
    delays: &[Duration],
) -> Outcome {
    match job {
        Job::Sprite(id, url) => Outcome::SpriteOnly {
            ok: fetch_sprite(source, cache, id, &url, delays),
        },
        Job::Full(id) => {
            let species: Species = match with_retry(delays, || source.species(id, types)) {
                Ok(s) => s,
                Err(_) => return Outcome::Failed,
            };
            if cache.write_species(&species).is_err() {
                return Outcome::Failed;
            }
            let sprite_ok = match &species.sprite_url {
                Some(url) => fetch_sprite(source, cache, id, url, delays),
                None => true,
            };
            Outcome::Saved { sprite_ok }
        }
    }
}

/// 캐시를 채운다. 목록조차 받지 못하면 `Err`.
///
/// `progress`는 호출한 스레드에서 `(처리 완료 수, 작업 총수)`로 불린다.
pub fn sync_pokedex(
    source: &dyn PokedexSource,
    cache: &dyn CacheWriter,
    workers: usize,
    retry_delays: &[Duration],
    progress: &mut dyn FnMut(usize, usize),
) -> Result<SyncReport, SourceError> {
    let ids = with_retry(retry_delays, || source.species_ids())?;
    let mut report = SyncReport {
        total: ids.len(),
        ..SyncReport::default()
    };

    // 타입 이름은 먼저 18종을 받는다. 실패한 타입은 이름 없이 둔다(화면에는 API 이름).
    let types: HashMap<String, LocalizedText> = PokemonType::ALL
        .iter()
        .filter_map(|t| {
            let name = t.api_name();
            with_retry(retry_delays, || source.type_names(name))
                .ok()
                .map(|n| (name.to_string(), n))
        })
        .collect();

    let mut jobs = VecDeque::new();
    for &id in &ids {
        if !cache.has_species(id) {
            jobs.push_back(Job::Full(id));
        } else {
            report.skipped += 1;
            if let Some(url) = cache.missing_sprite_url(id) {
                jobs.push_back(Job::Sprite(id, url));
            }
        }
    }

    let job_total = jobs.len();
    progress(0, job_total);

    let queue = Mutex::new(jobs);
    let workers = workers.clamp(1, MAX_WORKERS).min(job_total.max(1));
    let (tx, rx) = mpsc::channel::<Outcome>();
    let mut done = 0;

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let (queue, types) = (&queue, &types);
            scope.spawn(move || loop {
                let job = queue.lock().unwrap_or_else(|e| e.into_inner()).pop_front();
                let Some(job) = job else { break };
                if tx
                    .send(run_job(job, source, cache, types, retry_delays))
                    .is_err()
                {
                    break;
                }
            });
        }
        drop(tx);
        for outcome in rx {
            match outcome {
                Outcome::Saved { sprite_ok } => {
                    report.saved += 1;
                    report.sprites_failed += usize::from(!sprite_ok);
                }
                Outcome::Failed => report.failed += 1,
                Outcome::SpriteOnly { ok } => report.sprites_failed += usize::from(!ok),
            }
            done += 1;
            progress(done, job_total);
        }
    });

    let entries: Vec<IndexEntry> = cache.cached_index_entries();
    cache.write_index(&entries).map_err(|e| match e {
        CatalogError::Io(m) | CatalogError::Corrupt(m) => {
            SourceError::Permanent(format!("인덱스 저장 실패: {m}"))
        }
        other => SourceError::Permanent(format!("인덱스 저장 실패: {other:?}")),
    })?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::species::PokedexNumber;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const ZERO: [Duration; 2] = [Duration::ZERO; 2];

    // 합성 데이터.
    fn species(id: u32, sprite: bool) -> Species {
        Species {
            number: PokedexNumber(id),
            slug: format!("testmon-{id}"),
            name: LocalizedText {
                ko: None,
                en: Some(format!("Testmon {id}")),
            },
            genus: LocalizedText::default(),
            types: vec![],
            flavor_texts: vec![],
            sprite_url: sprite.then(|| format!("https://example.invalid/{id}.png")),
        }
    }

    #[derive(Default)]
    struct FakeSource {
        ids: Option<Vec<u32>>,
        /// 호출마다 앞에서 하나씩 꺼낸다. 비면 정상 종(스프라이트 있음)을 돌려준다.
        scripted: Mutex<HashMap<u32, VecDeque<Result<Species, SourceError>>>>,
        failing_sprites: HashSet<String>,
        species_calls: Mutex<Vec<u32>>,
        sprite_calls: AtomicUsize,
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        delay: Duration,
    }

    impl FakeSource {
        fn with_ids(ids: &[u32]) -> FakeSource {
            FakeSource {
                ids: Some(ids.to_vec()),
                ..Default::default()
            }
        }
        fn script(self, id: u32, results: Vec<Result<Species, SourceError>>) -> FakeSource {
            self.scripted.lock().unwrap().insert(id, results.into());
            self
        }
        fn calls_for(&self, id: u32) -> usize {
            self.species_calls
                .lock()
                .unwrap()
                .iter()
                .filter(|&&c| c == id)
                .count()
        }
    }

    impl PokedexSource for FakeSource {
        fn species_ids(&self) -> Result<Vec<u32>, SourceError> {
            self.ids
                .clone()
                .ok_or(SourceError::Transient("목록 실패".into()))
        }
        fn type_names(&self, api_name: &str) -> Result<LocalizedText, SourceError> {
            Ok(LocalizedText {
                ko: None,
                en: Some(api_name.to_uppercase()),
            })
        }
        fn species(
            &self,
            id: u32,
            _: &HashMap<String, LocalizedText>,
        ) -> Result<Species, SourceError> {
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_in_flight.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(self.delay);
            self.species_calls.lock().unwrap().push(id);
            let next = self
                .scripted
                .lock()
                .unwrap()
                .get_mut(&id)
                .and_then(VecDeque::pop_front);
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            next.unwrap_or_else(|| Ok(species(id, true)))
        }
        fn sprite(&self, url: &str) -> Result<Vec<u8>, SourceError> {
            self.sprite_calls.fetch_add(1, Ordering::SeqCst);
            if self.failing_sprites.contains(url) {
                Err(SourceError::Permanent("404".into()))
            } else {
                Ok(b"png".to_vec())
            }
        }
    }

    #[derive(Default)]
    struct FakeCache {
        species: Mutex<HashMap<u32, Species>>,
        sprites: Mutex<HashSet<u32>>,
        index: Mutex<Option<Vec<IndexEntry>>>,
    }

    impl FakeCache {
        fn with(list: Vec<Species>, sprites: &[u32]) -> FakeCache {
            let c = FakeCache::default();
            for s in list {
                c.species.lock().unwrap().insert(s.number.0, s);
            }
            c.sprites.lock().unwrap().extend(sprites);
            c
        }
    }

    impl CacheWriter for FakeCache {
        fn has_species(&self, id: u32) -> bool {
            self.species.lock().unwrap().contains_key(&id)
        }
        fn write_species(&self, s: &Species) -> Result<(), CatalogError> {
            self.species.lock().unwrap().insert(s.number.0, s.clone());
            Ok(())
        }
        fn write_sprite(&self, id: u32, _: &[u8]) -> Result<(), CatalogError> {
            self.sprites.lock().unwrap().insert(id);
            Ok(())
        }
        fn write_index(&self, entries: &[IndexEntry]) -> Result<(), CatalogError> {
            *self.index.lock().unwrap() = Some(entries.to_vec());
            Ok(())
        }
        fn cached_index_entries(&self) -> Vec<IndexEntry> {
            let mut v: Vec<IndexEntry> = self
                .species
                .lock()
                .unwrap()
                .values()
                .map(|s| IndexEntry {
                    id: s.number.0,
                    slug: s.slug.clone(),
                })
                .collect();
            v.sort_by_key(|e| e.id);
            v
        }
        fn missing_sprite_url(&self, id: u32) -> Option<String> {
            if self.sprites.lock().unwrap().contains(&id) {
                return None;
            }
            self.species.lock().unwrap().get(&id)?.sprite_url.clone()
        }
    }

    fn run(source: &FakeSource, cache: &FakeCache) -> Result<SyncReport, SourceError> {
        sync_pokedex(source, cache, 4, &ZERO, &mut |_, _| {})
    }

    #[test]
    fn saves_all_species_and_index() {
        let (source, cache) = (FakeSource::with_ids(&[1, 2, 3]), FakeCache::default());
        let r = run(&source, &cache).unwrap();
        assert_eq!(
            (r.total, r.saved, r.skipped, r.failed, r.sprites_failed),
            (3, 3, 0, 0, 0)
        );
        let index = cache.index.lock().unwrap().clone().unwrap();
        assert_eq!(
            index.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(cache.sprites.lock().unwrap().len(), 3);
    }

    #[test]
    fn type_names_are_passed_to_source() {
        struct Check(FakeSource);
        impl PokedexSource for Check {
            fn species_ids(&self) -> Result<Vec<u32>, SourceError> {
                self.0.species_ids()
            }
            fn type_names(&self, n: &str) -> Result<LocalizedText, SourceError> {
                self.0.type_names(n)
            }
            fn species(
                &self,
                id: u32,
                types: &HashMap<String, LocalizedText>,
            ) -> Result<Species, SourceError> {
                assert_eq!(types.len(), 18);
                assert_eq!(types["fire"].en.as_deref(), Some("FIRE"));
                self.0.species(id, types)
            }
            fn sprite(&self, url: &str) -> Result<Vec<u8>, SourceError> {
                self.0.sprite(url)
            }
        }
        let r = sync_pokedex(
            &Check(FakeSource::with_ids(&[1])),
            &FakeCache::default(),
            4,
            &ZERO,
            &mut |_, _| {},
        );
        assert_eq!(r.unwrap().saved, 1);
    }

    #[test]
    fn resume_skips_cached() {
        let source = FakeSource::with_ids(&[1, 2, 3]);
        let cache = FakeCache::with(vec![species(1, true), species(2, true)], &[1, 2]);
        let r = run(&source, &cache).unwrap();
        assert_eq!(*source.species_calls.lock().unwrap(), vec![3]);
        assert_eq!((r.saved, r.skipped), (1, 2));
        assert_eq!(cache.index.lock().unwrap().as_ref().unwrap().len(), 3);
    }

    #[test]
    fn transient_error_retried() {
        let source = FakeSource::with_ids(&[1, 2, 3]).script(
            2,
            vec![
                Err(SourceError::Transient("503".into())),
                Ok(species(2, true)),
            ],
        );
        let r = run(&source, &FakeCache::default()).unwrap();
        assert_eq!((r.saved, r.failed), (3, 0));
        assert_eq!(source.calls_for(2), 2);
    }

    #[test]
    fn transient_error_gives_up_after_retries() {
        let t = || Err(SourceError::Transient("503".into()));
        let source = FakeSource::with_ids(&[1, 2]).script(2, vec![t(), t(), t(), t()]);
        let r = run(&source, &FakeCache::default()).unwrap();
        assert_eq!((r.saved, r.failed), (1, 1));
        assert_eq!(source.calls_for(2), 3, "첫 시도 + 재시도 2회");
    }

    #[test]
    fn permanent_error_not_retried() {
        let source = FakeSource::with_ids(&[1, 2, 3])
            .script(2, vec![Err(SourceError::Permanent("404".into()))]);
        let cache = FakeCache::default();
        let r = run(&source, &cache).unwrap();
        assert_eq!((r.saved, r.failed), (2, 1));
        assert_eq!(source.calls_for(2), 1);
        assert!(!cache.has_species(2));
    }

    #[test]
    fn sprite_failure_keeps_species() {
        let mut source = FakeSource::with_ids(&[1, 2, 3]);
        source
            .failing_sprites
            .insert("https://example.invalid/2.png".into());
        let cache = FakeCache::default();
        let r = run(&source, &cache).unwrap();
        assert_eq!((r.saved, r.sprites_failed), (3, 1));
        assert!(cache.has_species(2));
        assert!(!cache.sprites.lock().unwrap().contains(&2));
    }

    #[test]
    fn missing_sprite_refetched_next_run() {
        let source = FakeSource::with_ids(&[1]);
        let cache = FakeCache::with(vec![species(1, true)], &[]);
        let r = run(&source, &cache).unwrap();
        assert!(source.species_calls.lock().unwrap().is_empty());
        assert_eq!(source.sprite_calls.load(Ordering::SeqCst), 1);
        assert!(cache.sprites.lock().unwrap().contains(&1));
        assert_eq!((r.saved, r.skipped), (0, 1));
    }

    #[test]
    fn null_sprite_url_not_refetched() {
        let source = FakeSource::with_ids(&[1, 2]).script(2, vec![Ok(species(2, false))]);
        let cache = FakeCache::with(vec![species(1, false)], &[]);
        run(&source, &cache).unwrap();
        assert_eq!(source.sprite_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn concurrency_capped_at_four() {
        let ids: Vec<u32> = (1..=40).collect();
        let source = FakeSource {
            delay: Duration::from_millis(5),
            ..FakeSource::with_ids(&ids)
        };
        let r = sync_pokedex(&source, &FakeCache::default(), 16, &ZERO, &mut |_, _| {}).unwrap();
        assert_eq!(r.saved, 40);
        let max = source.max_in_flight.load(Ordering::SeqCst);
        assert!((2..=4).contains(&max), "동시 실행 최대 {max}");
    }

    #[test]
    fn list_failure_is_error() {
        let source = FakeSource::default();
        assert!(run(&source, &FakeCache::default()).is_err());
    }

    #[test]
    fn progress_reports_total() {
        let source = FakeSource::with_ids(&[1, 2, 3, 4, 5]);
        let mut calls = Vec::new();
        sync_pokedex(&source, &FakeCache::default(), 4, &ZERO, &mut |d, t| {
            calls.push((d, t))
        })
        .unwrap();
        assert_eq!(calls.first(), Some(&(0, 5)));
        assert_eq!(calls.last(), Some(&(5, 5)));
        assert!(calls.windows(2).all(|w| w[0].0 <= w[1].0));
    }

    #[test]
    fn nothing_to_do_reports_zero_progress() {
        let source = FakeSource::with_ids(&[1]);
        let cache = FakeCache::with(vec![species(1, true)], &[1]);
        let mut calls = Vec::new();
        let r = sync_pokedex(&source, &cache, 4, &ZERO, &mut |d, t| calls.push((d, t))).unwrap();
        assert_eq!((r.saved, r.skipped), (0, 1));
        assert_eq!(calls, vec![(0, 0)]);
    }
}
