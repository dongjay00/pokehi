# 도메인 설계와 경계

> `SPEC.md`의 DDD 부분을 자세히 적은 문서다. 단계별 구현 계획(작업 순서와 테스트 목록)은 이 문서가 승인된 뒤 별도로 작성한다.

## 유비쿼터스 언어

| 용어 | 코드 이름 | 뜻 |
|---|---|---|
| 종 | `Species` | 도감 번호 하나에 대응하는 포켓몬. 인사 화면의 주인공이다. 폼 변종은 다루지 않는다. |
| 도감 번호 | `PokedexNumber` | 전국도감 번호. 1 이상이다. 화면에는 `No.025`처럼 3자리 이상, 0으로 채워 표시한다. |
| 언어 | `Language` | `Ko` 또는 `En`. 기본값은 `Ko`다. |
| 다국어 텍스트 | `LocalizedText` | 언어별 문자열 묶음. `get(lang)`은 요청 언어 → 영어 순으로 찾고, 둘 다 없으면 없음을 반환한다. |
| 타입 | `PokemonType` | 18개 타입. 고정된 열거형이며 타입별 뱃지 색을 가진다. 표시 이름은 `LocalizedText`다. |
| 분류 | `genus` | "쥐포켓몬" / "Mouse Pokémon" |
| 도감 설명 | `FlavorText` | 게임 버전별 설명 문구. 정규화(줄바꿈·폼피드·연속 공백 → 공백 하나)는 값 객체를 만들 때 한다. |
| 선택 방식 | `SelectionMode` | `Random` 또는 `Daily(NaiveDate)` |
| 스프라이트 | `Sprite` | RGBA 픽셀 격자. `crop_transparent()`와 `fit_within(w, h)`를 가진다. |
| 인사 | `Greeting` | 화면에 그릴 모든 값이 확정된 묶음이다. 들어가는 값은 번호, 이름, 보조 이름, 타입들, 분류, 설명 한 줄, 스프라이트(선택)다. |
| 동기화 | Sync | PokéAPI에서 캐시를 채우는 작업 |

## 계층과 의존 방향

```text
            ┌──────────────── cli (main.rs) ────────────────┐
            │ 옵션·환경변수 파싱, 언어 결정, 종료 코드, stdout/stderr │
            └───────┬──────────────────────────────┬───────┘
                    ▼                              ▼
          ShowGreeting 유스케이스            SyncPokedex 유스케이스
                    │                              │
        ┌───────────┼─────────────┐        ┌───────┴────────┐
        ▼           ▼             ▼        ▼                ▼
  SpeciesCatalog  SpriteStore  Clock/Rng  PokedexSource   CacheWriter
     (port)        (port)      (port)       (port)          (port)
        ▲           ▲                        ▲                ▲
  fs 캐시 어댑터 ───┘                   PokéAPI HTTP 어댑터   fs 캐시 어댑터

  domain: Species, LocalizedText, SelectionMode, select(), Greeting::compose(), Sprite
  render: Greeting → ANSI 문자열 (순수 함수, 터미널 폭을 인자로 받음)
```

- `domain`은 어떤 바깥 크레이트에도 의존하지 않는다. 예외는 날짜 타입과 표준 라이브러리 정도다.
- `render`는 순수 함수다. 입력은 `Greeting`과 터미널 폭이고, 출력은 `String`이다. 문자열 스냅샷으로 테스트한다.
- 포트는 trait로 정의한다. 유스케이스 테스트에는 메모리 가짜 구현을 쓴다.
- 어댑터만 HTTP, 파일시스템, PNG 디코딩을 안다.

## 핵심 규칙 (테스트로 먼저 정의할 것)

1. `LocalizedText::get`: ko가 있으면 ko, ko가 없으면 en, 둘 다 없으면 None
2. `FlavorText` 정규화: `"꼬리를 세우고 주변의\n상황을\f살피다"` → `"꼬리를 세우고 주변의 상황을 살피다"`
3. `select(Random, rng, ids)`: 주입한 rng에 따라 결정적으로 고르고, 빈 목록이면 None
4. `select(Daily(date), ids)`: 같은 날짜면 같은 결과, 여러 날짜에 걸쳐서는 고르게 퍼진다. 해시 함수는 Rust 버전이 바뀌어도 결과가 같아야 한다. 표준 `DefaultHasher`는 쓰지 않고 FNV, SplitMix 같은 고정 알고리즘을 쓴다.
5. 설명 문구 고르기: Daily 모드에서는 날짜 시드로 정하고, Random 모드에서는 rng로 정한다.
6. 표시 폭 계산: 한글을 비롯한 동아시아 전각 문자는 2칸으로 센다. 줄바꿈은 폭 기준으로 한다.
7. `Sprite::crop_transparent`: 알파 128 미만은 투명으로 본다. 전부 투명하면 빈 스프라이트가 된다.
8. `Sprite::fit_within(32, 32)`: 최근접 이웃 방식으로 줄이고, 가로세로 비율을 유지하며, 키우지는 않는다.
9. 반 블록 렌더링: 위아래 픽셀 쌍의 투명 여부에 따라 네 경우를 처리한다(둘 다 투명이면 공백, 위만 있으면 `▀`+전경색, 아래만 있으면 `▄`+전경색, 둘 다 있으면 `▀`+전경·배경색). 줄 끝마다 색을 리셋한다.
10. 색 모드 결정: `detect_color_mode(env, is_windows_vt_ok)`는 순수 함수다. 환경 변수 맵을 받아 SPEC의 결정 순서대로 `TrueColor | Ansi256 | None`를 반환한다.
11. 256색 변환: `to_ansi256(rgb)`는 6×6×6 큐브와 회색조 중 제곱 거리가 가장 가까운 번호를 고른다. 0~15번은 쓰지 않는다.
12. 배치: 정보 블록을 그림 높이 안에서 세로 가운데에 놓는다. 그림이 없으면 정보만 출력한다.

## 어댑터 메모

- **PokéAPI 어댑터**: `pokemon-species/{id}`에서 `names`, `genera`, `flavor_text_entries`, `varieties`를 뽑고, `pokemon/{id}`에서 `types`, `sprites.front_default`를 뽑는다. 타입 이름은 `type/{name}`에서 가져온다. 응답 JSON 픽스처는 `tests/fixtures/`에 둔다. 픽스처는 **합성 데이터**로, 실제 응답과 구조는 같지만 이름과 텍스트는 지어낸 것이다. 다음 세 경우를 갖춘다.
  - 한국어와 영어가 모두 있는 종
  - 한국어 도감 설명이 없는 종(실제로는 9세대가 이렇다)
  - 스프라이트 URL이 null인 종

  테스트용 PNG는 테스트 코드에서 생성한다. 실제 API를 부르는 확인용 테스트가 필요하면 `#[ignore]`로 두고 CI에서는 돌리지 않는다.
- **파일 캐시 어댑터**: 저장 형식은 구현 계획에서 정한다. 시작 경로에서 1025종 전체를 파싱하지 않아도 되도록 하는 게 요구 사항이다. 예를 들어 종별 작은 파일과 ID 목록 인덱스를 두는 방식이 있다.
- **터미널 폭**: 폭 감지는 CLI 계층에서 한다. 감지에 실패하면 80열로 본다.

## 크레이트 후보 [설계자 판단, 구현 계획에서 확정]

다음은 고정 사항이 아니라 출발점이다. 시작 시간 목표(50ms)와 비용 0원이라는 조건만 지키면 바꿔도 된다.

- CLI: `clap` (derive)
- HTTP: `ureq` (동기, 가벼움, sync에서만 사용)
- JSON: `serde`, `serde_json`
- 이미지: `image` (png 기능만) 또는 `png`
- 경로: `directories`
- 날짜: `chrono` 또는 `time`
- 난수: `fastrand` 또는 `rand`
- 문자 폭: `unicode-width`
- 터미널 폭: `terminal_size`
- Windows VT 모드: `enable-ansi-support`. Windows에서만 쓰는 의존성으로 두거나, `windows-sys`로 직접 호출한다.

## 플랫폼 경계

- 플랫폼별 코드(Windows VT 모드 켜기, TTY 감지, 데이터 디렉터리)는 CLI 계층과 어댑터에만 둔다. 도메인과 `render`는 OS를 모른다.
- `render`는 `ColorMode`를 인자로 받는다. 트루컬러와 256색은 이스케이프 시퀀스만 다르고 배치 로직은 같다.
