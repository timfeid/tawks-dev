use std::process::Command;

use serde_json::Value;

fn run_operator_pair_matrix() -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_run_gauntlet"))
        .args([
            "--operator-matrix",
            "--operators",
            "echo_signal,corruptor_of_data",
            "--matches",
            "6",
            "--jobs",
            "2",
        ])
        .output()
        .expect("run gauntlet binary");

    assert!(
        output.status.success(),
        "run_gauntlet failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("parse operator matrix report")
}

#[test]
fn supervised_operator_matrix_is_reproducible_for_echo_signal_and_corruptor() {
    let first = run_operator_pair_matrix();
    let second = run_operator_pair_matrix();

    assert_eq!(
        first, second,
        "supervised matrix output should be reproducible"
    );

    let termination_counts = first["balance"]["termination_counts"]
        .as_array()
        .expect("balance termination counts");
    assert!(
        termination_counts
            .iter()
            .all(|count| count["reason"] != "no_legal_action"),
        "the focused operator matrix should no longer report no_legal_action: {termination_counts:?}"
    );
}
