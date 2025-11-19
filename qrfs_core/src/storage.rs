use std::path::{Path, PathBuf};

use crate::disk::BlockId;
use crate::errors::QrfsError;

/// trait para cualquier backend de bloques (QRs, archivo grande, etc)
pub trait BlockStorage: Send + Sync {
    fn block_size(&self) -> usize;
    fn total_blocks(&self) -> u32;

    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError>;
    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError>;
}

/// implementacion base que luego hablara con imagenes PNG/QR
pub struct QrStorageManager {
    root_dir: PathBuf,
    block_size: usize,
    total_blocks: u32,
}

impl QrStorageManager {
    pub fn new(root_dir: impl Into<PathBuf>, block_size: usize, total_blocks: u32) -> Self {
        Self {
            root_dir: root_dir.into(),
            block_size,
            total_blocks,
        }
    }

    /// construye la ruta al archivo de imagen para un bloque dado
    /// ejemplo: qr_codes/042.png
    pub fn block_path(&self, id: BlockId) -> PathBuf {
        let filename = format!("{:03}.png", id);
        self.root_dir.join(filename)
    }

    fn ensure_root_exists(&self) -> Result<(), QrfsError> {
        std::fs::create_dir_all(&self.root_dir)?;
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

    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError> {
        // aca despues:
        // 1. Abrir PNG
        // 2. Leer QR
        // 3. Decodificar bytes
        let _path = self.block_path(id);
        Err(QrfsError::Unimplemented(format!(
            "read_block({id}) not implemented yet"
        )))
    }

    fn write_block(&self, id: BlockId, _data: &[u8]) -> Result<(), QrfsError> {
        // Aca despues:
        // 1. Codificar datos -> QR
        // 2. Guardar PNG
        self.ensure_root_exists()?;
        let _path = self.block_path(id);
        Err(QrfsError::Unimplemented(format!(
            "write_block({id}) not implemented yet"
        )))
    }
}
