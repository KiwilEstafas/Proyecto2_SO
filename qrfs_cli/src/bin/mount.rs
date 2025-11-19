use std::env;
use std::path::Path;
use std::process;
use std::sync::Arc;

use qrfs_core::errors::QrfsError;
use qrfs_core::fs::QrfsFilesystem;
use qrfs_core::storage::QrStorageManager;

fn main() {
    if let Err(e) = run() {
        eprintln!("mount.qrfs: error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), QrfsError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Uso: mount.qrfs <qrfolder/> <mountpoint/>");
        return Ok(());
    }

    let qrfolder = &args[1];
    let mountpoint = &args[2];

    // Estos parámetros deberían leerse del superblock en disco
    // aquí solo ponemos valores dummy
    let total_blocks = 128;
    let block_size = 4096;

    let storage = QrStorageManager::new(qrfolder, block_size, total_blocks);
    let fs = QrfsFilesystem::new(Arc::new(storage));

    let mountpoint_path = Path::new(mountpoint);

    // En el futuro: aquí se llamará a fs.mount() que internamente usará fuser::mount
    let _ = fs.mount(mountpoint_path)?;

    println!(
        "mount.qrfs: montaje de '{}' en '{}' (no implementado aún)",
        qrfolder, mountpoint
    );

    Ok(())
}
