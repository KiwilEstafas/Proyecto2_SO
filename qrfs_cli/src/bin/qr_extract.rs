// qr_extract - extrae los bloques qr de un archivo qrfs a una carpeta

use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::sync::Arc;

use qrfs_core::errors::QrfsError;
use qrfs_core::storage::{BlockStorage, QrStorageManager};
use qrfs_core::Superblock;

fn main() {
    if let Err(e) = run() {
        eprintln!("qrfs qr: error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), QrfsError> {
    let args: Vec<String> = env::args().collect();

    // parsear argumentos: qr_extract <qrfolder> <archivo_nombre> --out <output_dir>
    if args.len() < 5 {
        print_usage();
        return Ok(());
    }

    let qrfolder = &args[1];
    let file_identifier = &args[2]; // puede ser id de inodo o nombre
    
    // buscar --out
    let mut output_dir: Option<String> = None;
    let mut i = 3;
    while i < args.len() {
        if args[i] == "--out" && i + 1 < args.len() {
            output_dir = Some(args[i + 1].clone());
            break;
        }
        i += 1;
    }

    let output_dir = match output_dir {
        Some(dir) => dir,
        None => {
            eprintln!("qrfs qr: falta parametro --out <directorio>");
            print_usage();
            return Ok(());
        }
    };

    println!("qrfs qr: extrayendo bloques de '{}' a '{}'", file_identifier, output_dir);

    // cargar filesystem
    let block_size = 128;
    let total_blocks = 400;
    let storage = Arc::new(QrStorageManager::new(qrfolder, block_size, total_blocks));

    // leer superblock
    let sb_data = storage.read_block(0)?;
    let superblock: Superblock = bincode::deserialize(&sb_data)
        .map_err(|e| QrfsError::Other(format!("error leyendo superblock: {}", e)))?;

    if !superblock.is_valid() {
        return Err(QrfsError::Other("filesystem no valido".into()));
    }

    // cargar tabla de inodos
    let inodes = load_all_inodes(&storage, &superblock)?;
    
    // intentar parsear como id de inodo
    let target_inode = if let Ok(inode_id) = file_identifier.parse::<u32>() {
        // busqueda por id
        inodes
            .iter()
            .find(|inode| inode.id == inode_id)
            .ok_or_else(|| QrfsError::Other(format!("inodo {} no encontrado", inode_id)))?
    } else {
        // si no es numero, listar todos los archivos disponibles
        println!("qrfs qr: archivos disponibles en el filesystem:");
        println!();
        for inode in &inodes {
            let kind = match inode.kind {
                qrfs_core::InodeKind::File => "archivo",
                qrfs_core::InodeKind::Directory => "directorio",
            };
            println!("  inodo {}: {} ({} bloques, {} bytes)", 
                     inode.id, kind, inode.blocks.len(), inode.size);
        }
        println!();
        return Err(QrfsError::Other(
            format!("especifica el id del inodo a extraer (ej: qr_extract {} 2 --out {})", 
                    qrfolder, output_dir)
        ));
    };

    println!("qrfs qr: encontrado inodo {} con {} bloques", 
             target_inode.id, target_inode.blocks.len());
    
    // validar tamaño del archivo
    if target_inode.blocks.is_empty() {
        println!("qrfs qr: advertencia: el archivo no tiene bloques asignados (archivo vacio)");
        return Ok(());
    }
    
    // advertir si es archivo muy grande
    let estimated_qr_size = target_inode.blocks.len() * 10; // aproximadamente 10kb por qr
    if target_inode.blocks.len() > 100 {
        println!("qrfs qr: advertencia: archivo grande ({} bloques)", target_inode.blocks.len());
        println!("qrfs qr: tamaño estimado de salida: ~{} kb", estimated_qr_size);
    }
    
    println!();

    // crear directorio de salida
    fs::create_dir_all(&output_dir)
        .map_err(|e| QrfsError::Other(format!("error creando directorio: {}", e)))?;

    // extraer cada bloque
    let mut extracted_count = 0;
    let mut total_bytes = 0;
    let mut error_count = 0;

    for (idx, &block_id) in target_inode.blocks.iter().enumerate() {
        // obtener path del qr original
        let source_path = storage.block_path(block_id);
        
        if !source_path.exists() {
            println!("qrfs qr: error: bloque {} (id {}) no existe en disco", idx, block_id);
            error_count += 1;
            continue;
        }
        
        // leer tamaño del bloque para estadisticas
        match storage.read_block(block_id) {
            Ok(data) => {
                total_bytes += data.len();
            }
            Err(e) => {
                println!("qrfs qr: advertencia: no se pudo leer bloque {}: {}", idx, e);
            }
        }
        
        // copiar el qr directamente con nombre correlativo
        let output_filename = format!("block_{:04}.png", idx);
        let output_path = Path::new(&output_dir).join(output_filename);
        
        match fs::copy(&source_path, &output_path) {
            Ok(_) => {
                extracted_count += 1;
            }
            Err(e) => {
                println!("qrfs qr: error: no se pudo copiar qr {}: {}", idx, e);
                error_count += 1;
                continue;
            }
        }
        
        // progreso cada 10 bloques
        if (idx + 1) % 10 == 0 {
            println!("qrfs qr: extraidos {} de {} bloques...", idx + 1, target_inode.blocks.len());
        }
    }

    println!();
    println!("========================================");
    println!("extraccion completada:");
    println!("  bloques extraidos: {}", extracted_count);
    println!("  bloques con error: {}", error_count);
    println!("  tamaño total: {} bytes", total_bytes);
    println!("  directorio: {}", output_dir);
    
    if error_count > 0 {
        println!();
        println!("advertencia: {} bloques no pudieron ser procesados", error_count);
    }
    
    println!("========================================");

    Ok(())
}

fn print_usage() {
    eprintln!("uso:");
    eprintln!("  qr_extract <qrfolder> <id_inodo> --out <directorio_salida>");
    eprintln!();
    eprintln!("ejemplo:");
    eprintln!("  qr_extract disco_qr 2 --out ./qr_extraidos/");
    eprintln!();
    eprintln!("notas:");
    eprintln!("  - usa 'list' como id_inodo para ver todos los archivos disponibles");
    eprintln!("  - el inodo 0 es el directorio root");
    eprintln!("  - los archivos regulares empiezan desde el inodo 1 o 2");
}

// cargar todos los inodos del filesystem
fn load_all_inodes(
    storage: &Arc<QrStorageManager>,
    sb: &Superblock,
) -> Result<Vec<qrfs_core::Inode>, QrfsError> {
    let mut inodes = Vec::new();
    let mut inode_buffer = Vec::new();

    // leer bloques de la tabla de inodos
    for i in 0..sb.inode_table_blocks {
        let data = storage.read_block(sb.inode_table_start + i)?;
        inode_buffer.extend_from_slice(&data);
    }

    // deserializar inodos
    let mut cursor = std::io::Cursor::new(inode_buffer);
    for _ in 0..sb.inode_count {
        if let Ok(inode) = bincode::deserialize_from::<_, qrfs_core::Inode>(&mut cursor) {
            // solo inodos validos (mode != 0)
            if inode.mode != 0 {
                inodes.push(inode);
            }
        }
    }

    Ok(inodes)
}