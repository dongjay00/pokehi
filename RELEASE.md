# 배포 계획

> 비용 0원이 조건이다. 저장소는 GitHub 공개 저장소로 두고, 배포 채널은 crates.io와 GitHub Release 두 가지다.
> 이 문서는 계획이다.

## 배포 채널

| 채널 | 설치 방법 | 대상 |
|---|---|---|
| crates.io | `cargo install pokehi` | Rust가 설치된 사용자 |
| GitHub Release | OS별 미리 빌드한 바이너리를 내려받아 PATH에 둠 | Rust가 없는 사용자 |

GitHub Release용 바이너리 대상은 다음과 같다. **[설계자 판단]**
- `x86_64-unknown-linux-gnu` (WSL 포함)
- `aarch64-apple-darwin`, `x86_64-apple-darwin`
- `x86_64-pc-windows-msvc`

## CI (GitHub Actions, 공개 저장소 무료)

- 트리거는 push와 pull request다.
- ubuntu, macOS, windows에서 `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`를 돌린다.
- 테스트는 네트워크를 쓰지 않는다(SPEC 수용 기준 9).

## 릴리스 절차

1. `Cargo.toml`의 버전을 올린다.
2. CI가 통과하는지 확인한다.
3. `vX.Y.Z` 태그로 GitHub Release를 게시한다.
4. 릴리스 워크플로가 대상별 바이너리를 빌드해 Release에 첨부한다. 압축 파일과 SHA-256 체크섬을 함께 올린다.
5. 같은 워크플로가 crates.io에 `cargo publish`한다.
   - 인증은 crates.io Trusted Publishing(GitHub OIDC, `rust-lang/crates-io-auth-action`)을 쓴다. **[결정: 구현 시]**
   - Trusted Publisher는 crate가 crates.io에 이미 있어야 등록할 수 있다. 그래서 **첫 배포(0.1.0)는 로컬에서 API 토큰으로 직접 `cargo publish`** 한다. 그 뒤 crates.io의 crate 설정에서 Trusted Publisher(저장소 `dongjay00/pokehi`, 워크플로 `release.yml`)를 등록하면 다음 릴리스부터 워크플로가 배포한다.
   - 첫 릴리스 때는 워크플로의 `publish-crate` job이 실패할 수 있다(이미 배포된 버전). 바이너리 첨부(`build` job)와는 별개라서 Release 자체에는 영향이 없다.
6. 설치를 확인한다.

```sh
cargo install pokehi
pokehi sync
pokehi
```

## 일회성 설정 (사용자가 직접)

1. GitHub에 공개 저장소를 만든다: https://github.com/dongjay00/pokehi
2. crates.io 계정을 만든다(GitHub 로그인). API 토큰을 발급해 첫 버전을 직접 배포한 뒤 Trusted Publisher를 등록한다(위 릴리스 절차 5번).
3. 첫 릴리스 전에 crate 이름이 아직 비어 있는지 다시 확인한다. 2026-11-03에 crates.io API로 확인했을 때 `pokehi`는 비어 있었다(404). 이름은 언제든 선점될 수 있다.

## 배포물에 넣지 않는 것

- 포켓몬 그림, PokéAPI 데이터, 캐시 파일. 사용자가 `pokehi sync`로 직접 받는다(SPEC 반목표). 테스트 픽스처는 합성 데이터만 쓴다.

## README에 넣을 설치 안내

인사 화면은 대화형 셸에서만 뜨도록 안내한다. 비대화형 셸에서는 stdout이 터미널이 아니라서 어차피 출력되지 않지만, 한 번 더 막아 두는 용도다.

```sh
# macOS / Linux / WSL: ~/.zshrc 또는 ~/.bashrc
[[ $- == *i* ]] && command -v pokehi >/dev/null && pokehi
```

```powershell
# Windows PowerShell: notepad $PROFILE 로 열어서 추가
if (Get-Command pokehi -ErrorAction SilentlyContinue) { pokehi }
```

cmd.exe는 지원하지 않는다(SPEC "지원 터미널과 색 모드").

## 릴리스 전 수동 점검표

릴리스마다 아래 터미널에서 `pokehi --pokemon 25`와 `pokehi --pokemon 1000`을 실행한다. 그림, 색, 한글, 줄 맞춤을 확인하고 결과를 이 표에 기록한다.

| 터미널 | 색 모드(auto) | 버전 | 결과 | 확인일 |
|---|---|---|---|---|
| Windows Terminal + WSL | truecolor | | | |
| Windows 기본 콘솔 + PowerShell | truecolor | | | |
| macOS Terminal.app | 256 | | | |
| macOS iTerm2 또는 Ghostty | truecolor | | | |
| Linux GNOME Terminal 등 | truecolor | | | |
