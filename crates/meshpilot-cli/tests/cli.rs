use serde_json::Value;
use std::process::Command;

#[test]
fn status_never_maps_unknown_to_zero_or_offline() {
    let out = Command::new(env!("CARGO_BIN_EXE_meshpilot"))
        .args(["--dry-run", "status", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["read_only"], true);
    for key in [
        "online",
        "dns",
        "updatable_apps",
        "security_fixes",
        "last_success_utc",
    ] {
        assert_eq!(v.get(key), Some(&Value::Null), "{key}");
    }
}

#[test]
fn unsupported_write_command_is_not_success() {
    let out = Command::new(env!("CARGO_BIN_EXE_meshpilot"))
        .args(["updates", "apply"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn doctor_exit_matches_required_checks_even_outside_cachyos() {
    let out = Command::new(env!("CARGO_BIN_EXE_meshpilot"))
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    let checks = r["checks"].as_array().unwrap();
    assert!(
        checks
            .iter()
            .any(|c| c["id"] == "os" && c["required"] == true)
    );
    let ready = checks
        .iter()
        .all(|c| c["required"] != true || c["finding"] == "detected");
    assert_eq!(out.status.code(), Some(if ready { 0 } else { 3 }));
}
