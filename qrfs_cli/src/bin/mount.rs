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
    
    // sintaxis: mount.qrfs <qrfolder> <mountpoint>
    if args.len() != 3 {
        eprintln!("Uso: mount.qrfs <qrfolder/> <mountpoint/>");
        return Ok(());
    }

    let qrfolder = &args[1];
    let mountpoint = &args[2];

    println!("mount.qrfs: Montando '{}' en '{}'...", qrfolder, mountpoint);

    // configuracion estandar (debe coincidir con mkfs)
    let block_size = 128; 
    let total_blocks = 400; 

    // inicializar almacenamiento
    let storage = QrStorageManager::new(qrfolder, block_size, total_blocks);
    
    // inicializar Filesystem (esto lee la firma en el Bloque 0)
    let fs = QrfsFilesystem::new(Arc::new(storage))?;

    println!("mount.qrfs: Sistema listo. Presione Ctrl+C para desmontar.");
    
    // montar (bloquea la terminal)
    fs.mount(Path::new(mountpoint))?;

    Ok(())
}