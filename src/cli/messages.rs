//! 사용자에게 보여 주는 다국어 메시지.

use crate::domain::language::Language;

/// 메시지 종류.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    NoCache,
    VersionMismatch,
    Corrupt,
    BadArgs,
    BadLangEnv(String),
    BadMotionEnv(String),
    NotFound(String),
    SyncProgress(usize, usize),
    SyncDone {
        saved: usize,
        failed: usize,
        sprites_failed: usize,
    },
    SyncFailed(String),
    NothingFetched,
    NoDataDir,
}

/// 현재 언어로 메시지 문구를 만든다.
pub fn message(lang: Language, m: Msg) -> String {
    use Language::{En, Ko};
    match (lang, m) {
        (Ko, Msg::NoCache) => "pokehi: 먼저 'pokehi sync'를 실행하세요".into(),
        (En, Msg::NoCache) => "pokehi: run 'pokehi sync' first".into(),
        (Ko, Msg::VersionMismatch) => {
            "pokehi: 캐시 형식이 바뀌었습니다. 'pokehi sync'를 다시 실행하세요".into()
        }
        (En, Msg::VersionMismatch) => {
            "pokehi: cache format changed, run 'pokehi sync' again".into()
        }
        (Ko, Msg::Corrupt) => {
            "pokehi: 캐시를 읽을 수 없습니다. 'pokehi sync'를 다시 실행하세요".into()
        }
        (En, Msg::Corrupt) => "pokehi: cannot read cache, run 'pokehi sync' again".into(),
        (Ko, Msg::BadArgs) => "pokehi: 알 수 없는 옵션입니다. 'pokehi --help'를 확인하세요".into(),
        (En, Msg::BadArgs) => "pokehi: unknown option, see 'pokehi --help'".into(),
        (Ko, Msg::BadLangEnv(v)) => {
            format!("pokehi: POKEHI_LANG='{v}'을(를) 알 수 없어 ko를 사용합니다")
        }
        (En, Msg::BadLangEnv(v)) => format!("pokehi: unknown POKEHI_LANG='{v}', using ko"),
        (Ko, Msg::BadMotionEnv(v)) => {
            format!("pokehi: POKEHI_MOTION='{v}'을(를) 알 수 없어 모션 없이 표시합니다")
        }
        (En, Msg::BadMotionEnv(v)) => {
            format!("pokehi: unknown POKEHI_MOTION='{v}', showing without motion")
        }
        (Ko, Msg::NotFound(x)) => format!("pokehi: '{x}' 포켓몬이 캐시에 없습니다"),
        (En, Msg::NotFound(x)) => format!("pokehi: pokemon '{x}' not in cache"),
        (Ko, Msg::SyncProgress(d, t)) => format!("동기화 중 {d}/{t}"),
        (En, Msg::SyncProgress(d, t)) => format!("syncing {d}/{t}"),
        (
            Ko,
            Msg::SyncDone {
                saved,
                failed,
                sprites_failed,
            },
        ) => {
            format!("완료: {saved}종 저장, 실패 {failed}건, 그림 실패 {sprites_failed}건")
        }
        (
            En,
            Msg::SyncDone {
                saved,
                failed,
                sprites_failed,
            },
        ) => {
            format!("done: saved {saved}, failed {failed}, sprite failures {sprites_failed}")
        }
        (Ko, Msg::SyncFailed(e)) => format!("pokehi: 동기화 실패: {e}"),
        (En, Msg::SyncFailed(e)) => format!("pokehi: sync failed: {e}"),
        (Ko, Msg::NothingFetched) => "한 종도 받지 못했습니다".into(),
        (En, Msg::NothingFetched) => "no species could be fetched".into(),
        (Ko, Msg::NoDataDir) => {
            "pokehi: 데이터 디렉터리를 찾을 수 없습니다. POKEHI_DATA_DIR을 지정하세요".into()
        }
        (En, Msg::NoDataDir) => "pokehi: cannot find a data directory, set POKEHI_DATA_DIR".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_texts() {
        assert_eq!(
            message(Language::Ko, Msg::NoCache),
            "pokehi: 먼저 'pokehi sync'를 실행하세요"
        );
        assert_eq!(
            message(Language::En, Msg::NotFound("x".into())),
            "pokehi: pokemon 'x' not in cache"
        );
        assert_eq!(
            message(
                Language::Ko,
                Msg::SyncDone {
                    saved: 3,
                    failed: 1,
                    sprites_failed: 2
                }
            ),
            "완료: 3종 저장, 실패 1건, 그림 실패 2건"
        );
    }
}
