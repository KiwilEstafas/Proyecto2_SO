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
        eprintln!("Uso: mount.qrfs <carpeta_qr> <punto_montaje>");
        return Ok(());
    }

    let qrfolder = &args[1];
    let mountpoint = &args[2];

    // Asegúrate que estos valores sean iguales a los usados en mkfs
    let block_size = 128; 
    let total_blocks = 400; 

    let storage = QrStorageManager::new(qrfolder, block_size, total_blocks);
    let fs = QrfsFilesystem::new(Arc::new(storage))?;

    fs.mount(Path::new(mountpoint))?;

    Ok(())
}