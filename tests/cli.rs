//! 실제 바이너리를 실행하는 통합 테스트. 네트워크는 쓰지 않는다.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn pokehi(data_dir: &std::path::Path) -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("pokehi");
    cmd.env("POKEHI_DATA_DIR", data_dir)
        .env_remove("POKEHI_LANG")
        .env_remove("NO_COLOR");
    cmd
}

#[test]
fn help_exits_zero_and_mentions_sync() {
    let tmp = tempfile::tempdir().unwrap();
    pokehi(tmp.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sync"));
}

#[test]
fn piped_stdout_with_empty_cache_prints_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    pokehi(tmp.path()).assert().success().stdout("").stderr("");
}

#[test]
fn typo_option_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    pokehi(tmp.path())
        .arg("--tody")
        .assert()
        .success()
        .stdout("");
}

#[test]
fn forced_empty_cache_prints_hint() {
    let tmp = tempfile::tempdir().unwrap();
    pokehi(tmp.path())
        .arg("--force")
        .assert()
        .success()
        .stdout("")
        .stderr("pokehi: 먼저 'pokehi sync'를 실행하세요\n");
}

#[test]
fn sync_bad_option_exits_two() {
    let tmp = tempfile::tempdir().unwrap();
    pokehi(tmp.path()).args(["sync", "--x"]).assert().code(2);
}
