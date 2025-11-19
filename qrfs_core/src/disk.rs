use serde::{Deserialize, Serialize};

/// Identificador de bloque fisico (imagen QR)
pub type BlockId = u32;

/// numero magico para reconocer el FS.
pub const QRFS_MAGIC: u32 = 0x5152_4653; // "QRFS" en hex, mas o menos

/// Tamaño por defecto de bloque en bytes (AJUSTAR)
pub const DEFAULT_BLOCK_SIZE: u32 = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Superblock {
    pub magic: u32,
    pub block_size: u32,
    pub total_blocks: u32,

    /// Primer bloque reservado para la tabla de inodos
    pub inode_table_start: BlockId,
    /// Cantidad de inodos.
    pub inode_count: u32,

    /// Primer bloque de datos
    pub data_block_start: BlockId,
}

impl Superblock {
    pub fn new(total_blocks: u32, inode_count: u32) -> Self {
        let block_size = DEFAULT_BLOCK_SIZE;
        let inode_table_start = 1;
        let data_block_start = inode_table_start + (inode_count / 32).max(1); // heuristica tonta

        Superblock {
            magic: QRFS_MAGIC,
            block_size,
            total_blocks,
            inode_table_start,
            inode_count,
            data_block_start,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == QRFS_MAGIC
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InodeKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub id: u32,
    pub kind: InodeKind,
    pub size: u64,

    // Para simplificar, una lista de bloques directos
    // mas adelante  implementar bloques indirectos, etc
    pub blocks: Vec<BlockId>,
}

impl Inode {
    pub fn new(id: u32, kind: InodeKind) -> Self {
        Self {
            id,
            kind,
            size: 0,
            blocks: Vec::new(),
        }
    }
}

/// entrada de directorio: nombre -> numero de inodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub inode_id: u32,
}
