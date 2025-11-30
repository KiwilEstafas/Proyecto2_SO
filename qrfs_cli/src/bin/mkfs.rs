use std::env;
use std::process;

use qrfs_core::disk::{Superblock, BLOCK_SIZE};
use qrfs_core::errors::QrfsError;
use qrfs_core::fs_format::{create_empty_bitmap, create_inode_table, serialize_superblock};
use qrfs_core::storage::{BlockStorage, QrStorageManager};

fn main() {
    if let Err(e) = run() {
        eprintln!("mkfs.qrfs: error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), QrfsError> {
    let args: Vec<String> = env::args().collect();

    // Sintaxis: mkfs.qrfs <qrfolder> [--blocks N]
    if args.len() < 2 {
        eprintln!("Uso: mkfs.qrfs <qrfolder/> [--blocks N]");
        return Ok(());
    }

    let qr_folder = &args[1];
    let mut total_blocks = 400; // Valor por defecto seguro

    // Parseo manual de argumentos opcionales
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--blocks" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<u32>() {
                        total_blocks = n;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let inode_count = 64; // Cantidad fija de archivos soportados

    // 1. Crear e inicializar Superblock
    let superblock = Superblock::new(total_blocks, inode_count);
    if !superblock.is_valid() {
        return Err(QrfsError::Other("Error interno creando superblock".into()));
    }

    println!("mkfs.qrfs: Creando sistema de archivos en '{}'...", qr_folder);
    println!("  - Bloques Totales: {}", total_blocks);
    println!("  - Inodos Máximos:  {}", inode_count);

    let block_size = superblock.block_size as usize;
    let storage = QrStorageManager::new(qr_folder, block_size, superblock.total_blocks);

    // 2. Inicializar disco físico (imágenes vacías)
    storage.init_empty_blocks()?;

    // 3. Escribir Superblock (Bloque 0 - La "Firma" del inicio)
    let sb_bytes = serialize_superblock(&superblock)?;
    let mut sb_block = vec![0u8; block_size];
    sb_block[..sb_bytes.len()].copy_from_slice(&sb_bytes);
    storage.write_block(0, &sb_block)?;

    // 4. Escribir Bitmap
    let mut bitmap = create_empty_bitmap(superblock.total_blocks);
    // Marcar bloques reservados como usados
    for blk in 0..superblock.data_block_start {
        let byte = (blk / 8) as usize;
        let bit = (blk % 8) as u8;
        if byte < bitmap.len() { bitmap[byte] |= 1 << bit; }
    }
    
    let mut offset = 0;
    for i in 0..superblock.free_map_blocks {
        let mut blk_buf = vec![0u8; block_size];
        let end = usize::min(offset + block_size, bitmap.len());
        let len = end - offset;
        if len > 0 { blk_buf[..len].copy_from_slice(&bitmap[offset..end]); }
        storage.write_block(superblock.free_map_start + i, &blk_buf)?;
        offset += block_size;
    }

    // 5. Escribir Tabla de Inodos
    let inode_table_raw = create_inode_table(superblock.inode_count)?;
    let mut offset = 0;
    for i in 0..superblock.inode_table_blocks {
        let mut blk_buf = vec![0u8; block_size];
        if offset < inode_table_raw.len() {
            let end = usize::min(offset + block_size, inode_table_raw.len());
            let len = end - offset;
            if len > 0 { blk_buf[..len].copy_from_slice(&inode_table_raw[offset..end]); }
        }
        storage.write_block(superblock.inode_table_start + i, &blk_buf)?;
        offset += block_size;
    }

    println!("mkfs.qrfs: ¡Éxito! Sistema de archivos creado.");
    Ok(())
}