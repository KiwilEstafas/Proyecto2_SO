use std::env;
use std::process;

use qrfs_core::errors::QrfsError;
use qrfs_core::storage::QrStorageManager;

fn main() {
    if let Err(e) = run() {
        eprintln!("fsck.qrfs: error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), QrfsError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Uso: fsck.qrfs <qrfolder/>");
        return Ok(());
    }

    let qrfolder = &args[1];

    // Igual que antes, esto debe salir del superblock.
    let total_blocks = 128;
    let block_size = 4096;

    let _storage = QrStorageManager::new(qrfolder, block_size, total_blocks);

    // TODO:
    // 1. Leer superblock del bloque 0.
    // 2. Validar magic, rangos, mapas, etc.
    // 3. Recorrer inodos y verificar consistencia de bloques.

    println!("fsck.qrfs: chequeo de consistencia de '{}' (stub)", qrfolder);

    Ok(())
}
