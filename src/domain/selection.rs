//! 포켓몬 선택 규칙. 난수원과 날짜는 밖에서 주입한다.

use chrono::NaiveDate;

/// 포켓몬을 고르는 방식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// 실행할 때마다 무작위.
    Random,
    /// 오늘의 포켓몬. 같은 날짜에는 같은 결과.
    Daily(NaiveDate),
}

/// 도감 설명 문구를 고를 때 쓰는 salt("FLAV").
pub const FLAVOR_SALT: u64 = 0x464C_4156;

/// FNV-1a 64비트 해시. Rust 버전과 무관하게 결과가 고정된다.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// SplitMix64 한 단계. 입력에 황금비 상수를 더한 뒤 섞는다.
pub fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// 날짜(YYYY-MM-DD 문자열)에서 만든 결정적 시드.
pub fn daily_seed(date: NaiveDate) -> u64 {
    splitmix64(fnv1a64(date.format("%Y-%m-%d").to_string().as_bytes()))
}

/// `len`개 중 하나의 인덱스를 고른다. 비어 있으면 `None`.
///
/// - Random: `rng() % len`
/// - Daily: `salt`가 0이면 `daily_seed % len`, 아니면 `splitmix64(daily_seed ^ salt) % len`
///
/// salt 규약: 종 선택은 시도 횟수(0, 1, 2), 설명 문구 선택은 [`FLAVOR_SALT`].
pub fn pick_index(
    mode: &SelectionMode,
    len: usize,
    salt: u64,
    rng: &mut dyn FnMut() -> u64,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let value = match mode {
        SelectionMode::Random => rng(),
        SelectionMode::Daily(date) => {
            let seed = daily_seed(*date);
            if salt == 0 {
                seed
            } else {
                splitmix64(seed ^ salt)
            }
        }
    };
    Some((value % len as u64) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn fnv_known_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn splitmix_known_vector() {
        // 시드 0에서 SplitMix64가 내는 첫 값(표준 참조 구현과 동일).
        assert_eq!(splitmix64(0), 0xe220a8397b1dcdaf);
    }

    #[test]
    fn daily_seed_is_pinned() {
        // Rust 버전과 무관하게 고정되어야 한다.
        let d = date(2026, 11, 3);
        assert_eq!(fnv1a64(b"2026-11-03"), 0xd37bf833683634a2);
        assert_eq!(daily_seed(d), 0xcc3ad04ac68c1cc9);
        assert_eq!(
            pick_index(&SelectionMode::Daily(d), 1025, 0, &mut || unreachable!()),
            Some(206)
        );
    }

    #[test]
    fn daily_same_date_same_result() {
        let mode = SelectionMode::Daily(date(2026, 5, 17));
        let a = pick_index(&mode, 1025, 0, &mut || unreachable!());
        let b = pick_index(&mode, 1025, 0, &mut || unreachable!());
        assert_eq!(a, b);
    }

    #[test]
    fn daily_salt_changes_result() {
        let mode = SelectionMode::Daily(date(2026, 11, 3));
        let picks: std::collections::HashSet<_> = (0..3)
            .map(|salt| pick_index(&mode, 1025, salt, &mut || unreachable!()))
            .collect();
        assert_eq!(picks.len(), 3);
    }

    #[test]
    fn daily_spreads_over_a_year() {
        let start = date(2026, 1, 1);
        let picks: std::collections::HashSet<_> = (0..365)
            .map(|i| {
                let d = start + chrono::Duration::days(i);
                pick_index(&SelectionMode::Daily(d), 1025, 0, &mut || unreachable!()).unwrap()
            })
            .collect();
        assert!(picks.len() > 300, "서로 다른 인덱스 {}개", picks.len());
    }

    #[test]
    fn random_uses_injected_rng() {
        let mut vals = [7u64, 1030].into_iter();
        let mut rng = || vals.next().unwrap();
        assert_eq!(
            pick_index(&SelectionMode::Random, 1025, 0, &mut rng),
            Some(7)
        );
        assert_eq!(
            pick_index(&SelectionMode::Random, 1025, 0, &mut rng),
            Some(5)
        );
    }

    #[test]
    fn empty_len_is_none() {
        assert_eq!(pick_index(&SelectionMode::Random, 0, 0, &mut || 3), None);
        let daily = SelectionMode::Daily(date(2026, 1, 1));
        assert_eq!(pick_index(&daily, 0, 0, &mut || unreachable!()), None);
    }
}
