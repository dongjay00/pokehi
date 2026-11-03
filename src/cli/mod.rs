//! CLI 계층: 옵션·환경 변수 해석, 유스케이스 호출, 출력, 종료 코드.
//!
//! 모든 외부 입력(환경 변수, TTY 여부, 터미널 폭, 날짜, 난수, 원격 소스)은
//! `Context`로 주입되므로 테스트는 실제 터미널과 네트워크 없이 돈다.

pub mod messages;

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};

use crate::adapters::fs_cache::FsCache;
use crate::app::ports::PokedexSource;
use crate::app::show_greeting::{show_greeting, GreetingError, Target};
use crate::app::sync::{sync_pokedex, MAX_WORKERS};
use crate::domain::language::Language;
use crate::domain::selection::SelectionMode;
use crate::render::color::detect_color_mode;
use crate::render::{render, ColorMode};
use messages::{message, Msg};

/// 터미널 폭을 알 수 없을 때 쓰는 값.
const DEFAULT_WIDTH: usize = 80;
/// `sync`에서 일시적 실패를 다시 시도하기 전 기다리는 시간.
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(3)];

#[derive(Debug, Parser)]
#[command(
    name = "pokehi",
    version,
    about = "새 터미널을 열 때마다 포켓몬 한 마리가 인사합니다",
    after_help = "처음 한 번 'pokehi sync'로 도감 데이터를 받은 뒤, 셸 설정 파일에 'pokehi'를 넣으세요."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// 오늘의 포켓몬을 보여 줍니다(같은 날짜에는 같은 포켓몬)
    #[arg(long)]
    today: bool,

    /// 표시 언어 [기본: POKEHI_LANG 또는 ko]
    #[arg(long, value_enum, global = true)]
    lang: Option<LangArg>,

    /// 지정한 포켓몬을 보여 줍니다(도감 번호 또는 영문 이름)
    #[arg(long, value_name = "번호|영문이름")]
    pokemon: Option<String>,

    /// 색 출력 방식
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// stdout이 터미널이 아니어도 출력합니다
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// PokéAPI에서 도감 데이터와 그림을 받아 로컬 캐시에 저장합니다
    Sync,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LangArg {
    Ko,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ColorArg {
    Auto,
    Truecolor,
    #[value(name = "256")]
    Ansi256,
}

/// 실행 환경. `main.rs`가 실제 값을, 테스트가 가짜 값을 넣는다.
pub struct Context<'a> {
    pub env: HashMap<String, String>,
    pub stdout_is_tty: bool,
    pub term_width: Option<usize>,
    /// Windows에서 VT 모드를 켠 결과. 다른 OS에서는 `None`.
    pub windows_vt_ok: Option<bool>,
    pub today: NaiveDate,
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
    pub rng: Box<dyn FnMut() -> u64 + 'a>,
    pub make_source: Box<dyn Fn() -> Box<dyn PokedexSource> + 'a>,
}

impl Context<'_> {
    fn env_var(&self, key: &str) -> Option<&str> {
        self.env
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    /// stderr에 한 줄 쓴다. 쓰기 실패는 무시한다(셸 시작을 깨뜨리지 않기 위해).
    fn warn(&mut self, line: &str) {
        let _ = writeln!(self.err, "{line}");
    }
}

/// 언어를 정한다: `--lang` > `POKEHI_LANG` > ko. 환경 변수 값이 잘못됐으면 경고를 돌려준다.
fn resolve_lang(flag: Option<LangArg>, env: Option<&str>) -> (Language, Option<String>) {
    match (flag, env) {
        (Some(LangArg::Ko), _) => (Language::Ko, None),
        (Some(LangArg::En), _) => (Language::En, None),
        (None, None) => (Language::Ko, None),
        (None, Some(v)) => match Language::parse(v) {
            Some(lang) => (lang, None),
            None => (Language::Ko, Some(v.to_string())),
        },
    }
}

fn data_dir(ctx: &Context) -> Option<PathBuf> {
    if let Some(dir) = ctx.env_var("POKEHI_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("", "", "pokehi").map(|d| d.data_dir().to_path_buf())
}

/// 인자를 해석해 실행하고 종료 코드를 돌려준다.
pub fn run(args: Vec<OsString>, ctx: &mut Context) -> i32 {
    let is_sync = args.iter().skip(1).any(|a| a == "sync");
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(e) => return parse_error(e, is_sync, &args, ctx),
    };

    let (lang, bad_env) = resolve_lang(cli.lang, ctx.env_var("POKEHI_LANG"));
    let greeting_silent = cli.command.is_none() && !ctx.stdout_is_tty && !cli.force;
    if greeting_silent {
        // 비대화형 셸(scp, rsync 등)을 깨뜨리지 않도록 아무것도 쓰지 않는다.
        return 0;
    }
    if let Some(v) = bad_env {
        ctx.warn(&message(lang, Msg::BadLangEnv(v)));
    }

    let Some(dir) = data_dir(ctx) else {
        ctx.warn(&message(lang, Msg::NoDataDir));
        return if cli.command.is_some() { 1 } else { 0 };
    };

    match cli.command {
        Some(Command::Sync) => run_sync(FsCache::new(dir), lang, ctx),
        None => run_greeting(&cli, FsCache::new(dir), lang, ctx),
    }
}

fn parse_error(e: clap::Error, is_sync: bool, args: &[OsString], ctx: &mut Context) -> i32 {
    use clap::error::ErrorKind;
    if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
        let _ = write!(ctx.out, "{}", e.render());
        return 0;
    }
    if is_sync {
        let _ = write!(ctx.err, "{}", e.render());
        return 2;
    }
    // 인사 모드: .zshrc 오타로 셸이 실패 상태로 시작하지 않게 한 줄만 쓰고 0.
    let forced = args.iter().any(|a| a == "--force");
    if ctx.stdout_is_tty || forced {
        let (lang, _) = resolve_lang(None, ctx.env_var("POKEHI_LANG"));
        ctx.warn(&message(lang, Msg::BadArgs));
    }
    0
}

fn color_mode(cli: &Cli, ctx: &Context) -> ColorMode {
    let no_color = ctx.env_var("NO_COLOR").is_some();
    match cli.color {
        ColorArg::Truecolor if !no_color => ColorMode::TrueColor,
        ColorArg::Ansi256 if !no_color => ColorMode::Ansi256,
        ColorArg::Truecolor | ColorArg::Ansi256 => ColorMode::None,
        ColorArg::Auto => {
            let env = |k: &str| ctx.env.get(k).cloned();
            detect_color_mode(&env, ctx.windows_vt_ok)
        }
    }
}

fn run_greeting(cli: &Cli, cache: FsCache, lang: Language, ctx: &mut Context) -> i32 {
    let mode = color_mode(cli, ctx);
    let width = if ctx.stdout_is_tty {
        ctx.term_width.unwrap_or(DEFAULT_WIDTH)
    } else {
        DEFAULT_WIDTH
    };
    let target = match (&cli.pokemon, cli.today) {
        (Some(p), _) => Target::Specific(p.clone()),
        (None, true) => Target::Pick(SelectionMode::Daily(ctx.today)),
        (None, false) => Target::Pick(SelectionMode::Random),
    };

    match show_greeting(&cache, &cache, &target, lang, &mut *ctx.rng) {
        Ok(g) => {
            let _ = ctx.out.write_all(render(&g, width, mode).as_bytes());
            0
        }
        Err(GreetingError::NotFound(x)) => {
            ctx.warn(&message(lang, Msg::NotFound(x)));
            1
        }
        Err(e) => {
            let m = match e {
                GreetingError::NoCache => Msg::NoCache,
                GreetingError::VersionMismatch => Msg::VersionMismatch,
                _ => Msg::Corrupt,
            };
            ctx.warn(&message(lang, m));
            0
        }
    }
}

fn run_sync(cache: FsCache, lang: Language, ctx: &mut Context) -> i32 {
    let source = (ctx.make_source)();
    let err = &mut *ctx.err;
    let mut progress = |done: usize, total: usize| {
        let _ = write!(err, "\r{}", message(lang, Msg::SyncProgress(done, total)));
        let _ = err.flush();
    };
    let result = sync_pokedex(
        source.as_ref(),
        &cache,
        MAX_WORKERS,
        &RETRY_DELAYS,
        &mut progress,
    );
    let _ = writeln!(ctx.err);

    match result {
        Err(e) => {
            ctx.warn(&message(lang, Msg::SyncFailed(e.to_string())));
            1
        }
        Ok(r) => {
            let done = Msg::SyncDone {
                saved: r.saved,
                failed: r.failed,
                sprites_failed: r.sprites_failed,
            };
            let _ = writeln!(ctx.out, "{}", message(lang, done));
            if r.saved == 0 && r.skipped == 0 && r.failed > 0 {
                let reason = message(lang, Msg::NothingFetched);
                ctx.warn(&message(lang, Msg::SyncFailed(reason)));
                return 1;
            }
            0
        }
    }
}

#[cfg(test)]
mod tests;
