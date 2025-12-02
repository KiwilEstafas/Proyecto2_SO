use std::env;
use std::process;

use qrfs_core::disk::{Inode, Superblock, QRFS_MAGIC, QRFS_VERSION};
use qrfs_core::errors::QrfsError;
use qrfs_core::storage::{BlockStorage, QrStorageManager};
use std::collections::HashSet;

fn main() {
    if let Err(e) = run() {
        eprintln!("fsck.qrfs: error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), QrfsError> {
    let args: Vec<String> = env::args().collect();
    
    // sintaxis: fsck.qrfs <qrfolder>
    if args.len() != 2 {
        eprintln!("Uso: fsck.qrfs <qrfolder/>");
        return Ok(());
    }

    let qrfolder = &args[1];

    println!("fsck.qrfs: Iniciando verificación de '{}'", qrfolder);
    println!("--------------------------------------------------");

    let block_size = 128;
    let total_blocks = 400;

    let storage = QrStorageManager::new(qrfolder, block_size, total_blocks);

    // verificar superblock (firma)
    print!("[1/5] Verificando Superblock (Firma)... ");
    let superblock = check_superblock(&storage)?;
    println!("OK (Magic: {:X})", superblock.magic);

    // verificar limites del disco
    print!("[2/5] Verificando límites del disco... ");
    check_disk_layout(&superblock)?;
    println!("OK");

    // analizar bitmap de espacio
    print!("[3/5] Analizando Bitmap de espacio... ");
    let bitmap = load_bitmap(&storage, &superblock)?;
    println!("OK");

    // analizar tabla de inodos
    print!("[4/5] Analizando Tabla de Inodos... ");
    let inodes = load_inodes(&storage, &superblock)?;
    println!("OK ({} inodos activos)", inodes.len());

    // verificar consistencia bitmap vs inodos
    print!("[5/5] Verificando consistencia Bitmap vs Inodos... ");
    check_consistency(&bitmap, &inodes, &superblock)?;
    println!("OK");

    println!("--------------------------------------------------");
    println!("fsck.qrfs: El sistema de archivos está LIMPIO.");

    Ok(())
}

// funciones auxiliares de fsck 

fn check_superblock(storage: &QrStorageManager) -> Result<Superblock, QrfsError> {
    let data = storage.read_block(0)?;
    let sb: Superblock = bincode::deserialize(&data)
        .map_err(|_| QrfsError::Other("No se pudo leer el Superblock (Bloque 0)".into()))?;

    if sb.magic != QRFS_MAGIC {
        return Err(QrfsError::Other("Firma inválida (Magic Number incorrecto)".into()));
    }
    if sb.version != QRFS_VERSION {
        return Err(QrfsError::Other("Versión de QRFS no soportada".into()));
    }
    Ok(sb)
}

fn check_disk_layout(sb: &Superblock) -> Result<(), QrfsError> {
    if sb.data_block_start >= sb.total_blocks {
        return Err(QrfsError::Other("Layout corrupto: Inicio de datos fuera de rango".into()));
    }
    Ok(())
}

fn load_bitmap(storage: &QrStorageManager, sb: &Superblock) -> Result<Vec<u8>, QrfsError> {
    let mut bitmap = Vec::new();
    for i in 0..sb.free_map_blocks {
        let data = storage.read_block(sb.free_map_start + i)?;
        bitmap.extend_from_slice(&data);
    }
    Ok(bitmap)
}

fn load_inodes(storage: &QrStorageManager, sb: &Superblock) -> Result<Vec<Inode>, QrfsError> {
    let mut inodes = Vec::new();
    let mut buf = Vec::new();
    for i in 0..sb.inode_table_blocks {
        let data = storage.read_block(sb.inode_table_start + i)?;
        buf.extend_from_slice(&data);
    }
    let mut cursor = std::io::Cursor::new(buf);
    for _ in 0..sb.inode_count {
        if let Ok(inode) = bincode::deserialize_from::<_, Inode>(&mut cursor) {
            if inode.mode != 0 { inodes.push(inode); }
        }
    }
    Ok(inodes)
}

fn check_consistency(bitmap: &[u8], inodes: &[Inode], sb: &Superblock) -> Result<(), QrfsError> {
    let mut claimed_blocks = HashSet::new();
    
    // recolectar bloques reclamados por inodos
    for inode in inodes {
        for &blk in &inode.blocks {
            if blk >= sb.total_blocks {
                return Err(QrfsError::Other(format!("Inodo {} apunta a bloque fuera de rango {}", inode.id, blk)));
            }
            claimed_blocks.insert(blk);
        }
    }

    // verificar contra bitmap
    for blk in sb.data_block_start..sb.total_blocks {
        let byte = (blk / 8) as usize;
        let bit = (blk % 8) as u8;
        if byte >= bitmap.len() { break; }
        
        let is_used = (bitmap[byte] & (1 << bit)) != 0;
        let is_claimed = claimed_blocks.contains(&blk);

        if is_claimed && !is_used {
            return Err(QrfsError::Other(format!("CORRUPCIÓN: Bloque {} tiene datos pero está marcado como libre", blk)));
        }
    }
    Ok(())
}