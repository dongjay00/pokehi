# pokehi

새 터미널을 열 때마다 포켓몬 한 마리가 인사하는 작은 CLI입니다.

```text
 ▄▄▀▀▄▄          No.025 피카츄  Pikachu
▀█████▀▄         [전기]
 ██▀▀██          쥐포켓몬
  ▀  ▀           꼬리를 세우고 주변의 상황을 살피다 보면 가끔 꼬리에 번개가 친다.
```

왼쪽에는 색 반 블록 문자로 그린 포켓몬 그림이, 오른쪽에는 도감 번호·이름·타입·분류·도감 설명 한 줄이 나옵니다. 한국어가 기본이고 영어도 지원합니다.

인사 화면은 로컬 캐시만 읽고 네트워크를 쓰지 않으므로 셸 시작이 느려지지 않습니다. 오류가 나도 한 줄 안내만 출력하고 항상 종료 코드 0으로 끝나서 셸을 깨뜨리지 않습니다.

## 설치

Rust가 있다면:

```sh
cargo install pokehi
```

Rust가 없다면 [Releases](https://github.com/dongjay00/pokehi/releases)에서 OS에 맞는 파일을 내려받아 압축을 풀고, `pokehi`(Windows는 `pokehi.exe`)를 `PATH`에 있는 디렉터리에 넣으세요.

| 파일 | 대상 |
|---|---|
| `pokehi-<버전>-x86_64-unknown-linux-gnu.tar.gz` | Linux, WSL |
| `pokehi-<버전>-aarch64-apple-darwin.tar.gz` | macOS (Apple Silicon) |
| `pokehi-<버전>-x86_64-apple-darwin.tar.gz` | macOS (Intel) |
| `pokehi-<버전>-x86_64-pc-windows-msvc.zip` | Windows |

## 처음 한 번: 데이터 받기

```sh
pokehi sync
```

PokéAPI에서 전체 종 데이터와 그림을 받아 로컬 캐시에 저장합니다. 몇 분 정도 걸립니다. 이미 받은 항목은 건너뛰므로 중간에 끊겨도 다시 실행하면 이어서 받습니다.

## 셸 설정

인사 화면은 대화형 셸에서만 뜨게 해 두세요.

```sh
# macOS / Linux / WSL: ~/.zshrc 또는 ~/.bashrc
[[ $- == *i* ]] && command -v pokehi >/dev/null && pokehi
```

```powershell
# Windows PowerShell: notepad $PROFILE 로 열어서 추가
if (Get-Command pokehi -ErrorAction SilentlyContinue) { pokehi }
```

모션을 켜려면 `pokehi --motion hop`처럼 쓰거나 `export POKEHI_MOTION=hop`을 함께 넣으세요. 모션이 도는 동안(1초 미만)은 프롬프트가 늦게 뜹니다.

stdout이 터미널이 아니면 pokehi는 아무것도 출력하지 않습니다. 그래서 비대화형 SSH 세션의 `scp`, `rsync`, `sftp`도 깨지지 않습니다. cmd.exe는 지원하지 않습니다.

## 사용법

| 명령 | 동작 |
|---|---|
| `pokehi` | 랜덤 포켓몬 인사 화면 |
| `pokehi --today` | 오늘의 포켓몬. 같은 날짜에는 같은 포켓몬과 같은 문구 |
| `pokehi --lang en` | 영어로 표시. 기본은 `ko` |
| `pokehi --pokemon <번호\|영문이름>` | 지정한 포켓몬 표시(예: `--pokemon 25`, `--pokemon pikachu`) |
| `pokehi --color <auto\|truecolor\|256>` | 색 출력 방식. 기본은 `auto` |
| `pokehi --motion <hop\|shake\|none>` | 그림이 깡총 뛰거나(`hop`) 좌우로 흔들린 뒤 멈춤. 기본은 `none` |
| `pokehi --force` | stdout이 터미널이 아니어도 출력(파이프, 스크린샷용) |
| `pokehi sync` | 도감 데이터와 그림을 받아 캐시에 저장 |
| `pokehi --help`, `--version` | 사용법, 버전 |

### 환경 변수

| 변수 | 뜻 |
|---|---|
| `POKEHI_LANG` | 기본 언어(`ko` 또는 `en`). `--lang`이 더 우선합니다 |
| `POKEHI_MOTION` | 기본 모션(`hop`, `shake`, `none`). `--motion`이 더 우선합니다 |
| `POKEHI_DATA_DIR` | 캐시 위치를 바꿉니다 |
| `NO_COLOR` | 값이 있으면 색과 그림 없이 정보만 출력합니다 |

## 지원 터미널

| 환경 | 색 모드(`auto`) |
|---|---|
| Windows Terminal (WSL, PowerShell) | 트루컬러 |
| Windows 기본 콘솔의 PowerShell (Windows 10 이상) | 트루컬러 |
| macOS iTerm2, Ghostty, WezTerm, Kitty | 트루컬러 |
| macOS 기본 Terminal.app | 256색 |
| Linux GNOME Terminal, Konsole, Alacritty 등 | 트루컬러 |

색 모드는 환경 변수(`COLORTERM`, `WT_SESSION`, `TERM_PROGRAM`)만 보고 정합니다. 판단할 수 없으면 더 안전한 256색을 씁니다. 색이 이상하게 보이면 `--color 256`을 써 보세요.

## 캐시 위치

| OS | 기본 위치 |
|---|---|
| Linux | `$XDG_DATA_HOME/pokehi` 또는 `~/.local/share/pokehi` |
| macOS | `~/Library/Application Support/pokehi` |
| Windows | `%APPDATA%\pokehi\data` |

캐시 형식이 바뀐 새 버전으로 올리면 `pokehi sync`를 다시 실행하라는 안내가 나옵니다.

## 개발

```sh
cargo test                     # 네트워크 없이 도는 테스트
cargo clippy --all-targets -- -D warnings
cargo test live_fetch_pikachu -- --ignored   # 실제 PokéAPI 확인(선택)
```

릴리스는 `Cargo.toml` 버전을 올린 뒤 `vX.Y.Z` 태그를 push하면 GitHub Actions가 Release 생성, 바이너리 첨부, crates.io 배포까지 합니다. 자세한 절차는 [`RELEASE.md`](RELEASE.md)에 있습니다.

설계 문서는 [`SPEC.md`](SPEC.md)(제품 명세), [`PLAN.md`](PLAN.md)(도메인 설계), [`RELEASE.md`](RELEASE.md)(배포 계획)에 있습니다.

## 고지

- 이 프로젝트는 비공식 팬 프로젝트이며 Nintendo, Creatures, GAME FREAK, The Pokémon Company와 관련이 없습니다.
- 포켓몬 관련 이름과 이미지의 권리는 Nintendo/Creatures/GAME FREAK에 있습니다.
- pokehi는 포켓몬 데이터나 그림을 포함하거나 재배포하지 않습니다. 데이터는 `pokehi sync`를 실행한 사용자의 컴퓨터가 [PokéAPI](https://pokeapi.co/)에서 직접 받습니다.

## 라이선스

[MIT](LICENSE)
