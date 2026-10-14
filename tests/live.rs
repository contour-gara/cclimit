use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

#[test]
#[ignore = "reads the real Keychain and calls the real API"]
fn prints_real_limits() {
    Command::cargo_bin("cclimit")
        .unwrap()
        .assert()
        .success()
        .stdout(contains("5h ").and(contains("7d ")).and(contains("fable ")));
}
