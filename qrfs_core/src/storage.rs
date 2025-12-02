use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

// dependencias para qr e imagenes
use base64::{engine::general_purpose, Engine as _};
use image::Luma;
use qrcode::QrCode;
use rqrr;
use serde_json;

use crate::disk::BlockId;
use crate::errors::QrfsError;

pub trait BlockStorage: Send + Sync {
    fn block_size(&self) -> usize;
    fn total_blocks(&self) -> u32;
    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError>;
    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError>;
}

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

    pub fn init_empty_blocks(&self) -> Result<(), QrfsError> {
        let empty = vec![0u8; self.block_size];
        for id in 0..self.total_blocks {
            self.write_block(id as BlockId, &empty)?;
        }
        Ok(())
    }

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
    // lectura: png -> qr (texto json con metadata) -> bytes binarios
    // =========================================================
    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError> {
        self.check_range(id)?;
        let path = self.block_path(id);

        if !path.exists() {
            return Ok(vec![0u8; self.block_size]);
        }

        // 1. abrir imagen y convertir a grises
        let img_dynamic = image::open(&path)
            .map_err(|e| QrfsError::Other(format!("error abriendo imagen: {}", e)))?;
        let img_gray = img_dynamic.to_luma8();

        // 2. detectar qr
        let mut decoder = rqrr::PreparedImage::prepare(img_gray);
        let grids = decoder.detect_grids();
        if grids.is_empty() {
            return Err(QrfsError::Other(format!(
                "no se detecto qr en {}",
                path.display()
            )));
        }

        // 3. decodificar contenido (rqrr devuelve string utf-8)
        let (_meta, content_string) = grids[0]
            .decode()
            .map_err(|e| QrfsError::Other(format!("error decodificando qr (rqrr): {}", e)))?;

        // 4. intentar parsear como json con metadata
        let data = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content_string) {
            // tiene metadata con block_id y data
            if let Some(data_str) = parsed.get("data").and_then(|v| v.as_str()) {
                general_purpose::STANDARD
                    .decode(data_str)
                    .map_err(|e| QrfsError::Other(format!("error decodificando base64 desde metadata: {}", e)))?
            } else {
                // fallback: intentar decodificar el contenido directo como base64
                general_purpose::STANDARD
                    .decode(&content_string)
                    .map_err(|e| QrfsError::Other(format!("error decodificando base64: {}", e)))?
            }
        } else {
            // no es json, asumir que es base64 directo (compatibilidad con qrs viejos)
            general_purpose::STANDARD
                .decode(&content_string)
                .map_err(|e| QrfsError::Other(format!("error decodificando base64: {}", e)))?
        };

        // 5. ajustar tamanio al buffer esperado
        let mut result = data;
        if result.len() > self.block_size {
            result.truncate(self.block_size);
        }
        if result.len() < self.block_size {
            result.resize(self.block_size, 0);
        }

        Ok(result)
    }

    // =========================================================
    // escritura: bytes binarios -> json con metadata -> qr -> png
    // =========================================================
    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError> {
        self.check_range(id)?;

        if data.len() > self.block_size {
            return Err(QrfsError::Other(format!("datos muy grandes")));
        }

        // 1. codificar binario a base64
        let b64_string = general_purpose::STANDARD.encode(data);

        // 2. crear metadata con id del bloque (formato json compacto)
        let metadata = format!(r#"{{"block_id":{},"data":"{}"}}"#, id, b64_string);

        // 3. generar qr a partir del json
        let code = QrCode::new(metadata)
            .map_err(|e| QrfsError::Other(format!("error generando qr: {}", e)))?;

        // 4. renderizar
        let image = code
            .render::<Luma<u8>>()
            .min_dimensions(200, 200)
            .max_dimensions(200, 200)
            .build();

        // 5. guardar
        let path = self.block_path(id);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        image
            .save(&path)
            .map_err(|e| QrfsError::Other(format!("error guardando imagen: {}", e)))?;

        Ok(())
    }
}

// --- mantenemos el inmemorystorage igual ---
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
}

impl BlockStorage for InMemoryBlockStorage {
    fn block_size(&self) -> usize {
        self.block_size
    }
    fn total_blocks(&self) -> u32 {
        self.total_blocks
    }

    fn read_block(&self, id: BlockId) -> Result<Vec<u8>, QrfsError> {
        let offset = (id as usize) * self.block_size;
        if offset >= self.data.lock().unwrap().len() {
            return Err(QrfsError::Other("out of range".into()));
        }
        let end = offset + self.block_size;
        Ok(self.data.lock().unwrap()[offset..end].to_vec())
    }

    fn write_block(&self, id: BlockId, data: &[u8]) -> Result<(), QrfsError> {
        let offset = (id as usize) * self.block_size;
        let mut memory = self.data.lock().unwrap();
        if offset >= memory.len() {
            return Err(QrfsError::Other("out of range".into()));
        }
        let len = data.len().min(self.block_size);
        memory[offset..offset + len].copy_from_slice(&data[..len]);
        Ok(())
    }
}