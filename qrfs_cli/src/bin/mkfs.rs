use std::env;
use std::process;

use qrfs_core::disk::{Superblock, BLOCK_SIZE};
use qrfs_core::errors::QrfsError;
use qrfs_core::fs_format::{create_empty_bitmap, create_inode_table, serialize_superblock};
use qrfs_core::storage::QrStorageManager;
use qrfs_core::BlockStorage;

fn main() {
    if let Err(e) = run() {
        eprintln!("mkfs.qrfs: error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), QrfsError> {
    let args: Vec<String> = env::args().collect();

    // modo compat: mkfs.qrfs <qrfolder/>
    if args.len() == 2 && !args[1].starts_with('-') {
        let output = args[1].clone();
        return mkfs_with_params(Some(output), None, None);
    }

    if args.len() == 1 {
        print_usage();
        return Ok(());
    }

    let mut output: Option<String> = None;
    let mut blocks: Option<u32> = None;
    let mut size_bytes: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                if i + 1 >= args.len() {
                    eprintln!("mkfs.qrfs: falta valor para --output");
                    print_usage();
                    return Ok(());
                }
                output = Some(args[i + 1].clone());
                i += 2;
            }
            "--blocks" => {
                if i + 1 >= args.len() {
                    eprintln!("mkfs.qrfs: falta valor para --blocks");
                    print_usage();
                    return Ok(());
                }
                let v: u32 = args[i + 1].parse().map_err(|_| {
                    QrfsError::Other("valor invalido para --blocks".into())
                })?;
                blocks = Some(v);
                i += 2;
            }
            "--size" => {
                if i + 1 >= args.len() {
                    eprintln!("mkfs.qrfs: falta valor para --size");
                    print_usage();
                    return Ok(());
                }
                let v: u64 = args[i + 1].parse().map_err(|_| {
                    QrfsError::Other("valor invalido para --size".into())
                })?;
                size_bytes = Some(v);
                i += 2;
            }
            other => {
                eprintln!("mkfs.qrfs: flag desconocida {other}");
                print_usage();
                return Ok(());
            }
        }
    }

    mkfs_with_params(output, blocks, size_bytes)
}

fn print_usage() {
    eprintln!("Uso:");
    eprintln!("  mkfs.qrfs <qrfolder/>");
    eprintln!("  mkfs.qrfs --output DIR [--blocks N] [--size BYTES]");
}

fn mkfs_with_params(
    output: Option<String>,
    blocks: Option<u32>,
    size_bytes: Option<u64>,
) -> Result<(), QrfsError> {
    let output = match output {
        Some(o) => o,
        None => {
            eprintln!("mkfs.qrfs: falta --output o ruta de carpeta");
            print_usage();
            return Ok(());
        }
    };

    let total_blocks = if let Some(b) = blocks {
        b
    } else if let Some(sz) = size_bytes {
        let blk = (sz / BLOCK_SIZE as u64) as u32;
        if blk == 0 { 128 } else { blk }
    } else {
        128
    };

    let inode_count = 64;

    let superblock = Superblock::new(total_blocks, inode_count);
    if !superblock.is_valid() {
        return Err(QrfsError::Other(
            "superblock creado en estado invalido".into(),
        ));
    }

    let block_size = superblock.block_size as usize;
    let storage = QrStorageManager::new(&output, block_size, superblock.total_blocks);

    // inicializar todos los bloques vacios
    storage.init_empty_blocks()?;

    // escribir superblock en bloque 0
    let sb_bytes = serialize_superblock(&superblock)?;
    if sb_bytes.len() > block_size {
        return Err(QrfsError::Other(
            "superblock no cabe en un bloque".into(),
        ));
    }
    let mut sb_block = vec![0u8; block_size];
    sb_block[..sb_bytes.len()].copy_from_slice(&sb_bytes);
    storage.write_block(0, &sb_block)?;

    // generar bitmap de bloques libres
    let mut bitmap = create_empty_bitmap(superblock.total_blocks);

    // marcar bloques reservados como usados
    for blk in 0..superblock.data_block_start {
        let byte = (blk / 8) as usize;
        let bit = (blk % 8) as u8;
        if byte < bitmap.len() {
            bitmap[byte] |= 1 << bit;
        }
    }

    // escribir bitmap en los bloques reservados
    let mut offset = 0;
    for i in 0..superblock.free_map_blocks {
        let mut blk_buf = vec![0u8; block_size];
        let end = usize::min(offset + block_size, bitmap.len());
        let len = end - offset;
        if len > 0 {
            blk_buf[..len].copy_from_slice(&bitmap[offset..end]);
        }
        storage.write_block(superblock.free_map_start + i, &blk_buf)?;
        offset += block_size;
    }

    // generar tabla de inodos inicial
    let inode_table_raw = create_inode_table(superblock.inode_count)?;

    let max_inode_bytes = superblock.inode_table_blocks as usize * block_size;
    if inode_table_raw.len() > max_inode_bytes {
        return Err(QrfsError::Other(
            "tabla de inodos no cabe en el espacio reservado".into(),
        ));
    }

    // escribir tabla de inodos en los bloques reservados
    let mut offset = 0;
    for i in 0..superblock.inode_table_blocks {
        let mut blk_buf = vec![0u8; block_size];
        
        // CORRECCIÓN: Solo intentamos copiar si aún quedan datos
        if offset < inode_table_raw.len() {
            let end = usize::min(offset + block_size, inode_table_raw.len());
            let len = end - offset;
            
            if len > 0 {
                blk_buf[..len].copy_from_slice(&inode_table_raw[offset..end]);
            }
        }
        
        // Escribimos el bloque (sea con datos parciales o totalmente vacío/relleno de ceros)
        storage.write_block(superblock.inode_table_start + i, &blk_buf)?;
        
        offset += block_size;
    }

    // validacion basica leyendo el superblock del disco
    let read_back = storage.read_block(0)?;
    let expected = serialize_superblock(&superblock)?;
    if expected.len() > read_back.len()
        || read_back[..expected.len()] != expected[..]
    {
        return Err(QrfsError::Other(
            "verificacion de superblock fallo".into(),
        ));
    }

    println!(
        "mkfs.qrfs: creado QRFS en '{}' con {} bloques y {} inodos",
        output, total_blocks, inode_count
    );

    Ok(())
}
