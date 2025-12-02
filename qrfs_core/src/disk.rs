use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// identificador de bloque
pub type BlockId = u32;

// tamaño fijo del bloque logico
pub const BLOCK_SIZE: usize = 128;

// numero magico qrfs
pub const QRFS_MAGIC: u32 = 0x5152_4653;

// version del formato qrfs
pub const QRFS_VERSION: u32 = 1;

// tipos de inodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InodeKind {
    File,
    Directory,
}

// estructura del inodo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub id: u32,
    pub kind: InodeKind,

    pub size: u64,

    // bloques directos
    pub blocks: Vec<BlockId>,

    // permisos estilo unix simplificados
    pub mode: u16,

    // timestamps unix
    pub created_at: u64,
    pub modified_at: u64,
}

impl Inode {
    pub fn new(id: u32, kind: InodeKind) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            kind,
            size: 0,
            blocks: Vec::new(),
            mode: 0o755,
            created_at: now,
            modified_at: now,
        }
    }
}

// representa una entrada dentro de una carpeta 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub inode_id: u32,
    pub kind: InodeKind,
}

// superblock qrfs
// bloque 0 contiene esta estructura serializada con bincode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Superblock {
    pub magic: u32,
    pub version: u32,

    pub block_size: u32,
    pub total_blocks: u32,

    // inicio del bitmap
    pub free_map_start: BlockId,
    pub free_map_blocks: u32,

    // inicio de la tabla de inodos
    pub inode_table_start: BlockId,
    pub inode_count: u32,
    pub inode_table_blocks: u32,

    // inodo root
    pub root_inode: u32,

    // inicio de los bloques de datos
    pub data_block_start: BlockId,
}

impl Superblock {
    pub fn new(total_blocks: u32, inode_count: u32) -> Self {
        // bloque 0 siempre es superblock
        let block_size = BLOCK_SIZE as u32;
        let free_map_start = 1;

        let free_map_blocks = 1;

        let inode_table_start = free_map_start + free_map_blocks;

        // calcular cuantos bloques necesitamos para los inodos
        let bytes_per_inode = 80;
        let total_inode_bytes = inode_count * bytes_per_inode;

        // division techo (ceiling division) para asegurar que quepan
        let inode_table_blocks = (total_inode_bytes + block_size - 1) / block_size;

        let data_block_start = inode_table_start + inode_table_blocks;

        Self {
            magic: QRFS_MAGIC,
            version: QRFS_VERSION,
            block_size,
            total_blocks,
            free_map_start,
            free_map_blocks,
            inode_table_start,
            inode_count,
            inode_table_blocks,
            root_inode: 0,
            data_block_start,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == QRFS_MAGIC && self.version == QRFS_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superblock_basic_layout_is_consistent() {
        let sb = Superblock::new(128, 64);

        assert_eq!(sb.magic, QRFS_MAGIC);
        assert_eq!(sb.version, QRFS_VERSION);
        assert!(sb.is_valid());

        assert_eq!(sb.block_size as usize, BLOCK_SIZE);
        assert_eq!(sb.free_map_start, 1);
        assert_eq!(sb.free_map_blocks, 1);

        assert_eq!(sb.inode_table_start, sb.free_map_start + sb.free_map_blocks);
        assert_eq!(
            sb.data_block_start,
            sb.inode_table_start + sb.inode_table_blocks
        );
    }

    #[test]
    fn inode_new_has_default_values() {
        let inode = Inode::new(10, InodeKind::File);

        assert_eq!(inode.id, 10);
        matches!(inode.kind, InodeKind::File);
        assert_eq!(inode.size, 0);
        assert!(inode.blocks.is_empty());
        assert_eq!(inode.mode, 0o755);

        assert!(inode.created_at > 0);
        assert!(inode.modified_at >= inode.created_at);
    }
}