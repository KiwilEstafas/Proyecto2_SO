use std::collections::HashSet;
use std::env;
use std::process;

use qrfs_core::disk::{Inode, Superblock, QRFS_MAGIC, QRFS_VERSION};
use qrfs_core::errors::QrfsError;
use qrfs_core::storage::{BlockStorage, QrStorageManager};

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

    println!("fsck.qrfs: iniciando verificacion de '{}'", qrfolder);
    println!();

    // estos valores deben coincidir con mkfs
    let block_size = 128;
    let total_blocks = 400;

    let storage = QrStorageManager::new(qrfolder, block_size, total_blocks);

    // paso 1: leer y validar superblock
    println!("[1/6] verificando superblock...");
    let superblock = check_superblock(&storage)?;
    println!("  ok: magic=0x{:08X}, version={}, bloques={}", 
             superblock.magic, superblock.version, superblock.total_blocks);
    println!();

    // paso 2: verificar estructura del disco
    println!("[2/6] verificando estructura del disco...");
    check_disk_layout(&superblock)?;
    println!("  ok: layout consistente");
    println!();

    // paso 3: cargar y validar bitmap
    println!("[3/6] verificando bitmap de bloques libres...");
    let bitmap = load_bitmap(&storage, &superblock)?;
    let bitmap_stats = analyze_bitmap(&bitmap, &superblock);
    println!("  bloques totales: {}", superblock.total_blocks);
    println!("  bloques usados:  {}", bitmap_stats.used_blocks);
    println!("  bloques libres:  {}", bitmap_stats.free_blocks);
    println!();

    // paso 4: cargar y validar inodos
    println!("[4/6] verificando tabla de inodos...");
    let inodes = load_inodes(&storage, &superblock)?;
    println!("  inodos cargados: {}", inodes.len());
    
    let mut has_errors = false;
    for inode in &inodes {
        if let Err(e) = validate_inode(inode, &superblock) {
            println!("  error en inodo {}: {}", inode.id, e);
            has_errors = true;
        }
    }
    
    if !has_errors {
        println!("  ok: todos los inodos son validos");
    }
    println!();

    // paso 5: verificar root directory
    println!("[5/6] verificando directorio raiz...");
    check_root_inode(&inodes, &superblock)?;
    println!("  ok: inodo raiz existe y es valido");
    println!();

    // paso 6: validar coherencia bitmap vs inodos
    println!("[6/6] verificando coherencia bitmap vs inodos...");
    let coherence = check_bitmap_coherence(&bitmap, &inodes, &superblock);
    
    if !coherence.orphan_blocks.is_empty() {
        println!("  advertencia: {} bloques marcados como usados pero no asignados a ningun inodo",
                 coherence.orphan_blocks.len());
        if coherence.orphan_blocks.len() <= 5 {
            println!("    bloques: {:?}", coherence.orphan_blocks);
        }
    }
    
    if !coherence.missing_blocks.is_empty() {
        println!("  error: {} bloques asignados a inodos pero marcados como libres en bitmap",
                 coherence.missing_blocks.len());
        if coherence.missing_blocks.len() <= 5 {
            println!("    bloques: {:?}", coherence.missing_blocks);
        }
        has_errors = true;
    }
    
    if coherence.orphan_blocks.is_empty() && coherence.missing_blocks.is_empty() {
        println!("  ok: bitmap coherente con inodos");
    }
    println!();

    // resumen final
    println!("========================================");
    if has_errors {
        println!("resultado: errores criticos encontrados");
        println!("el filesystem puede estar corrupto");
        process::exit(2);
    } else if !coherence.orphan_blocks.is_empty() {
        println!("resultado: advertencias menores encontradas");
        println!("el filesystem es usable pero tiene inconsistencias");
    } else {
        println!("resultado: filesystem consistente");
        println!("no se encontraron errores");
    }

    Ok(())
}

// carga y valida el superblock del bloque 0
fn check_superblock(storage: &QrStorageManager) -> Result<Superblock, QrfsError> {
    let data = storage.read_block(0)?;
    
    let superblock: Superblock = bincode::deserialize(&data)
        .map_err(|e| QrfsError::Other(format!("no se pudo deserializar superblock: {}", e)))?;

    // validar magic number
    if superblock.magic != QRFS_MAGIC {
        return Err(QrfsError::Other(format!(
            "magic number invalido: esperado 0x{:08X}, encontrado 0x{:08X}",
            QRFS_MAGIC, superblock.magic
        )));
    }

    // validar version
    if superblock.version != QRFS_VERSION {
        return Err(QrfsError::Other(format!(
            "version no soportada: esperada {}, encontrada {}",
            QRFS_VERSION, superblock.version
        )));
    }

    // validar que block_size es correcto
    if superblock.block_size != 128 && superblock.block_size != 256 && superblock.block_size != 512 {
        return Err(QrfsError::Other(format!(
            "block_size invalido: {}", superblock.block_size
        )));
    }

    Ok(superblock)
}

// verifica que la distribucion del disco sea logica
fn check_disk_layout(sb: &Superblock) -> Result<(), QrfsError> {
    // el superblock siempre empieza en 0
    if sb.free_map_start == 0 {
        return Err(QrfsError::Other(
            "bitmap no puede empezar en bloque 0 (reservado para superblock)".into()
        ));
    }

    // la tabla de inodos debe estar despues del bitmap
    if sb.inode_table_start < sb.free_map_start + sb.free_map_blocks {
        return Err(QrfsError::Other(
            "tabla de inodos se solapa con bitmap".into()
        ));
    }

    // los datos deben empezar despues de los inodos
    if sb.data_block_start < sb.inode_table_start + sb.inode_table_blocks {
        return Err(QrfsError::Other(
            "bloques de datos se solapan con tabla de inodos".into()
        ));
    }

    // verificar que no excedemos el total de bloques
    if sb.data_block_start >= sb.total_blocks {
        return Err(QrfsError::Other(
            "data_block_start excede el total de bloques del disco".into()
        ));
    }

    Ok(())
}

// carga el bitmap completo del disco
fn load_bitmap(storage: &QrStorageManager, sb: &Superblock) -> Result<Vec<u8>, QrfsError> {
    let mut bitmap = Vec::new();

    for i in 0..sb.free_map_blocks {
        let data = storage.read_block(sb.free_map_start + i)?;
        bitmap.extend_from_slice(&data);
    }

    // recortar al tamaño exacto
    let total_bytes = (sb.total_blocks as usize + 7) / 8;
    if bitmap.len() > total_bytes {
        bitmap.truncate(total_bytes);
    }

    Ok(bitmap)
}

struct BitmapStats {
    used_blocks: u32,
    free_blocks: u32,
}

// analiza el bitmap y cuenta bloques usados/libres
fn analyze_bitmap(bitmap: &[u8], sb: &Superblock) -> BitmapStats {
    let mut used = 0;
    let mut free = 0;

    for (byte_idx, byte) in bitmap.iter().enumerate() {
        for bit in 0..8 {
            let global_bit = byte_idx * 8 + bit;
            if global_bit >= sb.total_blocks as usize {
                break;
            }

            if (byte & (1 << bit)) != 0 {
                used += 1;
            } else {
                free += 1;
            }
        }
    }

    BitmapStats {
        used_blocks: used,
        free_blocks: free,
    }
}

// carga todos los inodos validos del disco
fn load_inodes(storage: &QrStorageManager, sb: &Superblock) -> Result<Vec<Inode>, QrfsError> {
    let mut inodes = Vec::new();
    let mut inode_buffer = Vec::new();

    // leer todos los bloques de la tabla de inodos
    for i in 0..sb.inode_table_blocks {
        let data = storage.read_block(sb.inode_table_start + i)?;
        inode_buffer.extend_from_slice(&data);
    }

    // deserializar inodos secuencialmente
    let mut cursor = std::io::Cursor::new(inode_buffer);
    for _ in 0..sb.inode_count {
        if let Ok(inode) = bincode::deserialize_from::<_, Inode>(&mut cursor) {
            // solo considerar inodos validos (mode != 0 indica que esta en uso)
            if inode.mode != 0 {
                inodes.push(inode);
            }
        }
    }

    Ok(inodes)
}

// valida un inodo individual
fn validate_inode(inode: &Inode, sb: &Superblock) -> Result<(), QrfsError> {
    // verificar que el id este en rango
    if inode.id >= sb.inode_count {
        return Err(QrfsError::Other(format!(
            "id fuera de rango: {} >= {}",
            inode.id, sb.inode_count
        )));
    }

    // verificar que todos los bloques asignados esten en el rango de datos
    for &block_id in &inode.blocks {
        if block_id < sb.data_block_start {
            return Err(QrfsError::Other(format!(
                "bloque {} esta en area reservada (data_block_start={})",
                block_id, sb.data_block_start
            )));
        }
        if block_id >= sb.total_blocks {
            return Err(QrfsError::Other(format!(
                "bloque {} excede total_blocks ({})",
                block_id, sb.total_blocks
            )));
        }
    }

    // verificar timestamps basicos
    if inode.created_at == 0 {
        return Err(QrfsError::Other("timestamp created_at es 0".into()));
    }

    if inode.modified_at < inode.created_at {
        return Err(QrfsError::Other(
            "modified_at es anterior a created_at".into()
        ));
    }

    Ok(())
}

// verifica que el inodo root exista y sea valido
fn check_root_inode(inodes: &[Inode], sb: &Superblock) -> Result<(), QrfsError> {
    let root_id = sb.root_inode;

    let root = inodes
        .iter()
        .find(|i| i.id == root_id)
        .ok_or_else(|| QrfsError::Other(format!("inodo root {} no encontrado", root_id)))?;

    // el root debe ser un directorio
    if !matches!(root.kind, qrfs_core::disk::InodeKind::Directory) {
        return Err(QrfsError::Other(
            "inodo root no es un directorio".into()
        ));
    }

    Ok(())
}

struct CoherenceResult {
    orphan_blocks: Vec<u32>,    // bloques marcados como usados pero no asignados
    missing_blocks: Vec<u32>,   // bloques asignados pero marcados como libres
}

// verifica coherencia entre bitmap y los bloques asignados a inodos
fn check_bitmap_coherence(
    bitmap: &[u8],
    inodes: &[Inode],
    sb: &Superblock,
) -> CoherenceResult {
    // recolectar todos los bloques asignados a inodos
    let mut assigned_blocks = HashSet::new();
    for inode in inodes {
        for &block_id in &inode.blocks {
            assigned_blocks.insert(block_id);
        }
    }

    let mut orphan_blocks = Vec::new();
    let mut missing_blocks = Vec::new();

    // recorrer todos los bloques del area de datos
    for block_id in sb.data_block_start..sb.total_blocks {
        let byte_idx = (block_id / 8) as usize;
        let bit = (block_id % 8) as u8;

        if byte_idx >= bitmap.len() {
            break;
        }

        let is_used_in_bitmap = (bitmap[byte_idx] & (1 << bit)) != 0;
        let is_assigned = assigned_blocks.contains(&block_id);

        if is_used_in_bitmap && !is_assigned {
            // bloque marcado como usado pero no asignado a ningun inodo
            orphan_blocks.push(block_id);
        } else if !is_used_in_bitmap && is_assigned {
            // bloque asignado a inodo pero marcado como libre (error critico)
            missing_blocks.push(block_id);
        }
    }

    CoherenceResult {
        orphan_blocks,
        missing_blocks,
    }
}