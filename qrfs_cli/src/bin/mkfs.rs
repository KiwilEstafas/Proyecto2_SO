use std::env;
use std::process;

use qrfs_core::disk::Superblock;
use qrfs_core::storage::QrStorageManager;
use qrfs_core::errors::QrfsError;

fn main() {
    if let Err(e) = run() {
        eprintln!("mkfs.qrfs: error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), QrfsError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Uso: mkfs.qrfs <qrfolder/>");
        return Ok(());
    }

    let qrfolder = &args[1];

    // Parámetros de ejemplo. Más adelante leerlos de flags
    let total_blocks = 128;
    let inode_count = 64;

    let superblock = Superblock::new(total_blocks, inode_count);
    if !superblock.is_valid() {
        return Err(QrfsError::Other(
            "superblock created in invalid state".into(),
        ));
    }

    // Creamos el storage manager (aún sin lógica real).
    let _storage = QrStorageManager::new(qrfolder, superblock.block_size as usize, total_blocks);

    // TODO:
    // 1. Serializar superblock e inodos iniciales.
    // 2. Escribirlos en los bloques QR usando storage.write_block().

    println!(
        "mkfs.qrfs: inicialización lógica de QRFS en '{}' ({} bloques, {} inodos)",
        qrfolder, total_blocks, inode_count
    );

    Ok(())
}
