use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("synopsis")
}

fn setup_project(src: &str) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname=\"test\"\nversion=\"0.1.0\"\nedition=\"2024\"",
    )
    .unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let file = src_dir.join("lib.rs");
    fs::write(&file, src).unwrap();
    (tmp, file)
}

#[test]
fn index_and_get_file() {
    let src = r#"
/// Greets someone.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}")
}

pub struct Config {
    pub debug: bool,
}
"#;
    let (tmp, file) = setup_project(src);
    let bin = binary();

    let index = Command::new(&bin)
        .args(["index", file.to_str().unwrap()])
        .output()
        .expect("failed to run binary");
    assert!(
        index.status.success(),
        "index failed: {:?}",
        String::from_utf8_lossy(&index.stderr)
    );

    let get = Command::new(&bin)
        .args(["get", file.to_str().unwrap()])
        .output()
        .expect("failed to run binary");
    assert!(get.status.success());

    let json: serde_json::Value = serde_json::from_slice(&get.stdout).expect("invalid JSON output");
    let items = json["items"].as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|i| i["name"] == "greet" && i["kind"] == "fn")
    );
    assert!(
        items
            .iter()
            .any(|i| i["name"] == "Config" && i["kind"] == "struct")
    );

    let greet_item = items.iter().find(|i| i["name"] == "greet").unwrap();
    assert_eq!(greet_item["docs"], "Greets someone.");
    assert!(
        greet_item["signature"]
            .as_str()
            .unwrap()
            .contains("pub fn greet")
    );

    drop(tmp);
}

#[test]
fn get_specific_item_returns_raw_source() {
    let src = "pub fn target_fn(x: u32) -> u32 { x * 2 }\npub fn other() {}";
    let (_tmp, file) = setup_project(src);
    let bin = binary();

    let target = format!("{}::target_fn", file.to_str().unwrap());
    let get = Command::new(&bin)
        .args(["get", &target])
        .output()
        .expect("failed to run binary");
    assert!(
        get.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&get.stderr)
    );

    let output = String::from_utf8_lossy(&get.stdout);
    assert!(output.contains("target_fn"), "got: {output}");
    assert!(
        !output.contains("other"),
        "should not contain other fn: {output}"
    );
}

#[test]
fn get_auto_indexes_unindexed_file() {
    let src = "pub const ANSWER: u32 = 42;";
    let (_tmp, file) = setup_project(src);
    let bin = binary();

    // No index run — get should auto-index
    let get = Command::new(&bin)
        .args(["get", file.to_str().unwrap()])
        .output()
        .expect("failed to run binary");
    assert!(
        get.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&get.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&get.stdout).unwrap();
    assert!(
        json["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["name"] == "ANSWER")
    );
}

#[test]
fn reindex_unchanged_file_does_not_rewrite_cache() {
    let src = "pub fn stable() {}";
    let (tmp, file) = setup_project(src);
    let bin = binary();

    Command::new(&bin)
        .args(["index", file.to_str().unwrap()])
        .output()
        .unwrap();

    let cache_path = tmp.path().join(".ast-cache/files/src/lib.rs.json");
    let mtime1 = fs::metadata(&cache_path).unwrap().modified().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(50));

    Command::new(&bin)
        .args(["index", file.to_str().unwrap()])
        .output()
        .unwrap();
    let mtime2 = fs::metadata(&cache_path).unwrap().modified().unwrap();

    assert_eq!(mtime1, mtime2, "cache was rewritten despite no change");
}
