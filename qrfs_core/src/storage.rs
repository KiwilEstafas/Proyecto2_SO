use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

// Dependencias para QR e Imágenes
use qrcode::QrCode;
use image::Luma; 
use rqrr; 

use crate::disk::BlockId;
use crate::errors::QrfsError;

/// trait para cualquier backend de bloques
pub trait BlockStorage: Send + Sync {
    fn block_size(&self) -> usize;
    fn total_blocks(&self) -> u32;

    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError>;
    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError>;
}

/// Implementación REAL que usa archivos QR en disco
pub struct QrStorageManager {
    root_dir: PathBuf,
    block_size: usize,
    total_blocks: u32,
}

impl QrStorageManager {
    pub fn new(root_dir: impl Into<PathBuf>, block_size: usize, total_blocks: u32) -> Self {
        let root_dir = root_dir.into();
        if let Err(e) = fs::create_dir_all(&root_dir) {
            eprintln!("qrfs: warning: no se pudo crear el directorio raiz: {e}");
        }

        Self {
            root_dir,
            block_size,
            total_blocks,
        }
    }

    /// Inicializa bloques vacíos (ahora genera imágenes QR vacías)
    pub fn init_empty_blocks(&self) -> Result<(), QrfsError> {
        let empty = vec![0u8; self.block_size];
        for id in 0..self.total_blocks {
            self.write_block(id as BlockId, &empty)?;
        }
        Ok(())
    }

    /// Construye la ruta al archivo PNG
    pub fn block_path(&self, id: BlockId) -> PathBuf {
        let filename = format!("{:06}.png", id);
        self.root_dir.join(filename)
    }

    fn check_range(&self, id: BlockId) -> Result<(), QrfsError> {
        if id >= self.total_blocks {
            return Err(QrfsError::Other(format!(
                "block id {id} fuera de rango 0..{}",
                self.total_blocks - 1
            )));
        }
        Ok(())
    }
}

impl BlockStorage for QrStorageManager {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn total_blocks(&self) -> u32 {
        self.total_blocks
    }

    // =========================================================
    // LECTURA: Imagen PNG -> Bytes
    // =========================================================
    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError> {
        self.check_range(id)?;

        let path = self.block_path(id);

        if !path.exists() {
            return Ok(vec![0u8; self.block_size]);
        }

        // Mapear ImageError a QrfsError::Other
        let img_dynamic = image::open(&path)
            .map_err(|e| QrfsError::Other(format!("Error abriendo imagen: {}", e)))?;
        
        let img_gray = img_dynamic.to_luma8();

        let mut decoder = rqrr::PreparedImage::prepare(img_gray);
        
        let grids = decoder.detect_grids();
        if grids.is_empty() {
            return Err(QrfsError::Other(format!("No se detectó QR en {}", path.display())));
        }

        let (_meta, content) = grids[0].decode().map_err(|e| 
            QrfsError::Other(format!("Error decodificando QR: {}", e))
        )?;

        let mut data = content.into_bytes();
        
        // Ajuste de tamaño si es necesario
        if data.len() > self.block_size {
            data.truncate(self.block_size);
        }
        if data.len() < self.block_size {
            data.resize(self.block_size, 0);
        }

        Ok(data)
    }

    // =========================================================
    // ESCRITURA: Bytes -> Imagen PNG
    // =========================================================
    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError> {
        self.check_range(id)?;

        if data.len() > self.block_size {
            return Err(QrfsError::Other(format!(
                "write_block datos demasiado grandes {} > {}",
                data.len(),
                self.block_size
            )));
        }

        let code = QrCode::new(data).map_err(|e| 
            QrfsError::Other(format!("Error generando QR: {}", e))
        )?;

        let image = code.render::<Luma<u8>>()
            .min_dimensions(200, 200)
            .max_dimensions(200, 200)
            .build();

        let path = self.block_path(id);
        
        if let Some(parent) = path.parent() {
             let _ = fs::create_dir_all(parent);
        }

        // CORREGIDO: Mapeamos ImageError a QrfsError::Other
        image.save(&path)
            .map_err(|e| QrfsError::Other(format!("Error guardando imagen: {}", e)))?;
        
        Ok(())
    }
}

/// Implementación de almacenamiento en memoria para pruebas
pub struct InMemoryBlockStorage {
    block_size: usize,
    total_blocks: u32,
    data: Mutex<Vec<u8>>,
}

impl InMemoryBlockStorage {
    pub fn new(total_blocks: u32, block_size: usize) -> Self {
        let len = total_blocks as usize * block_size;
        Self {
            block_size,
            total_blocks,
            data: Mutex::new(vec![0u8; len]),
        }
    }

    fn check_range(&self, id: BlockId) -> Result<usize, QrfsError> {
        if id >= self.total_blocks {
            return Err(QrfsError::Other(format!(
                "block id {id} fuera de rango 0..{}",
                self.total_blocks - 1
            )));
        }
        let offset = id as usize * self.block_size;
        Ok(offset)
    }
}

impl BlockStorage for InMemoryBlockStorage {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn total_blocks(&self) -> u32 {
        self.total_blocks
    }

    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError> {
        let offset = self.check_range(id)?;
        let end = offset + self.block_size;

        let data = self.data.lock().unwrap();
        Ok(data[offset..end].to_vec())
    }

    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError> {
        if data.len() > self.block_size {
            return Err(QrfsError::Other(format!(
                "write_block datos demasiado grandes {} > {}",
                data.len(),
                self.block_size
            )));
        }

        let offset = self.check_range(id)?;
        let end = offset + self.block_size;

        let mut data_vec = self.data.lock().unwrap();
        let slice = &mut data_vec[offset..end];

        let to_copy = data.len();
        slice[..to_copy].copy_from_slice(data);

        // Padding
        if to_copy < self.block_size {
            for b in &mut slice[to_copy..] {
                *b = 0;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::BLOCK_SIZE;

    fn temp_dir() -> std::path::PathBuf {
        let base = std::env::temp_dir();
        let unique = format!("qrfs_storage_test_{}", std::process::id());
        let dir = base.join(unique);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn inmemory_roundtrip_read_write() {
        let storage = InMemoryBlockStorage::new(4, BLOCK_SIZE);

        let data = b"hola qrfs";
        storage.write_block(1, data).unwrap();

        let read = storage.read_block(1).unwrap();
        assert_eq!(&read[..data.len()], data);
        assert_eq!(read.len(), BLOCK_SIZE);
    }

    #[test]
    fn inmemory_out_of_range_fails() {
        let storage = InMemoryBlockStorage::new(2, BLOCK_SIZE);
        let res = storage.read_block(5);
        assert!(res.is_err());
    }

    #[test]
    fn qrstorage_init_creates_images() {
        let dir = temp_dir();
        let storage = QrStorageManager::new(&dir, BLOCK_SIZE, 4);

        storage.init_empty_blocks().unwrap();

        for id in 0..4 {
            let path = storage.block_path(id);
            assert!(path.exists());
            assert_eq!(path.extension().unwrap(), "png");
        }
    }
}