use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::disk::{BlockId, BLOCK_SIZE};
use crate::errors::QrfsError;

/// trait para cualquier backend de bloques
pub trait BlockStorage: Send + Sync {
    fn block_size(&self) -> usize;
    fn total_blocks(&self) -> u32;

    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError>;
    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError>;
}

/// implementacion base que luego hablara con imagenes png qr
/// por ahora cada bloque se mapea a un archivo binario en disco
pub struct QrStorageManager {
    root_dir: PathBuf,
    block_size: usize,
    total_blocks: u32,
}

impl QrStorageManager {
    /// crea un manejador de bloques apuntando a un directorio
    /// no inicializa los bloques, solo asegura que el directorio exista
    pub fn new(root_dir: impl Into<PathBuf>, block_size: usize, total_blocks: u32) -> Self {
        let root_dir = root_dir.into();
        if let Err(e) = std::fs::create_dir_all(&root_dir) {
            eprintln!("qrfs: warning: no se pudo crear el directorio raiz: {e}");
        }

        Self {
            root_dir,
            block_size,
            total_blocks,
        }
    }

    /// inicializa todos los bloques en disco con ceros
    /// pensado para ser usado por mkfs
    pub fn init_empty_blocks(&self) -> Result<(), QrfsError> {
        let empty = vec![0u8; self.block_size];
        for id in 0..self.total_blocks {
            self.write_block(id as BlockId, &empty)?;
        }
        Ok(())
    }

    /// construye la ruta al archivo que representa un bloque
    pub fn block_path(&self, id: BlockId) -> PathBuf {
        let filename = format!("{:06}.blk", id);
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

    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError> {
        self.check_range(id)?;

        let path = self.block_path(id);
        let mut buf = vec![0u8; self.block_size];

        // si no existe el archivo, se asume bloque lleno de ceros
        if !path.exists() {
            return Ok(buf);
        }

        let mut file = File::open(path)?;
        let n = file.read(&mut buf)?;
        if n < self.block_size {
            for i in n..self.block_size {
                buf[i] = 0;
            }
        }

        Ok(buf)
    }

    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError> {
        self.check_range(id)?;

        if data.len() > self.block_size {
            return Err(QrfsError::Other(format!(
                "write_block datos demasiado grandes {} > {}",
                data.len(),
                self.block_size
            )));
        }

        let path = self.block_path(id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        file.write_all(data)?;

        if data.len() < self.block_size {
            let padding_len = self.block_size - data.len();
            let padding = vec![0u8; padding_len];
            file.write_all(&padding)?;
        }

        Ok(())
    }
}
