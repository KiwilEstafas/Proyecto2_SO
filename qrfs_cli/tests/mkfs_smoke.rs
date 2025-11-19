use std::fs;
use std::path::PathBuf;
use std::process::Command;

use qrfs_core::disk::BLOCK_SIZE;

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let unique = format!("qrfs_mkfs_test_{}", std::process::id());
    let dir = base.join(unique);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn mkfs_creates_block_zero_with_expected_size() {
    let dir = temp_dir();
    let qrfolder = dir.join("fs");

    let status = Command::new(env!("CARGO_BIN_EXE_mkfs"))
        .arg(&qrfolder)
        .status()
        .expect("no se pudo ejecutar mkfs");

    assert!(status.success());

    let block0 = qrfolder.join("000000.blk");
    assert!(block0.exists());

    let data = fs::read(block0).unwrap();
    assert_eq!(data.len(), BLOCK_SIZE);
}
