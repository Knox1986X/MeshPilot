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
    let on_cachyos = checks
        .iter()
        .any(|c| c["id"] == "os" && c["finding"] == "detected");
    for id in ["desktop-webkitgtk", "desktop-javascriptcoregtk"] {
        let check = checks.iter().find(|c| c["id"] == id);
        assert_eq!(check.is_some(), on_cachyos);
        if let Some(check) = check {
            assert_eq!(check["required"], false, "окно не обязательно для CLI");
        }
    }
    let ready = checks
        .iter()
        .all(|c| c["required"] != true || c["finding"] == "detected");
    assert_eq!(out.status.code(), Some(if ready { 0 } else { 3 }));
}

#[test]
fn first_run_leaves_user_directories_untouched() {
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = env::temp_dir().join(format!("meshpilot-test-{}-{stamp}", std::process::id()));
    fs::create_dir(&home).unwrap();
    for subcommand in ["status", "doctor"] {
        let result = Command::new(env!("CARGO_BIN_EXE_meshpilot"))
            .arg(subcommand)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_STATE_HOME", home.join("state"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .output()
            .unwrap();
        assert!(matches!(result.status.code(), Some(0 | 3)));
        assert_eq!(fs::read_dir(&home).unwrap().count(), 0);
    }
    fs::remove_dir(home).unwrap();
}
