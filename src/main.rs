//! 진입점. 실제 환경(환경 변수, TTY, 터미널 폭, 날짜, 난수, HTTP 소스)을 모아 `cli::run`에 넘긴다.

use std::io::{BufWriter, IsTerminal, Write};

use pokehi::adapters::pokeapi::UreqSource;
use pokehi::cli::{run, Context};

#[cfg(windows)]
fn enable_vt() -> Option<bool> {
    Some(enable_ansi_support::enable_ansi_support().is_ok())
}

#[cfg(not(windows))]
fn enable_vt() -> Option<bool> {
    None
}

fn main() {
    let stdout = std::io::stdout();
    let stdout_is_tty = stdout.is_terminal();
    let mut out = BufWriter::new(stdout.lock());
    let mut err = std::io::stderr();

    let env = std::env::vars_os()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.to_string_lossy().into_owned(),
            )
        })
        .collect();

    let size = terminal_size::terminal_size();
    let code = {
        let mut ctx = Context {
            env,
            stdout_is_tty,
            term_width: size.map(|(w, _)| usize::from(w.0)),
            term_height: size.map(|(_, h)| usize::from(h.0)),
            windows_vt_ok: enable_vt(),
            today: chrono::Local::now().date_naive(),
            out: &mut out,
            err: &mut err,
            rng: Box::new(|| fastrand::u64(..)),
            sleep: Box::new(std::thread::sleep),
            make_source: Box::new(|| Box::new(UreqSource::new())),
        };
        run(std::env::args_os().collect(), &mut ctx)
    };

    let _ = out.flush();
    drop(out);
    std::process::exit(code);
}
