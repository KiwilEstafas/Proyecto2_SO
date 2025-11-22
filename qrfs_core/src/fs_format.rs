use crate::disk::{Inode, InodeKind, Superblock};
use crate::errors::QrfsError;
use crate::disk::BLOCK_SIZE;

/// Genera un vector de bytes representando el bitmap
pub fn create_empty_bitmap(total_blocks: u32) -> Vec<u8> {
    let bytes = (total_blocks as usize + 7) / 8;
    vec![0u8; bytes]
}

/// Serializa superblock a bytes
pub fn serialize_superblock(sb: &Superblock) -> Result<Vec<u8>, QrfsError> {
    let encoded = bincode::serialize(sb)?;
    Ok(encoded)
}

/// Serializa la tabla de inodos inicial
pub fn create_inode_table(count: u32) -> Result<Vec<u8>, QrfsError> {
    let mut inodes = Vec::new();

    for i in 0..count {
        let kind = if i == 0 {
            InodeKind::Directory
        } else {
            InodeKind::File
        };

        let mut inode = Inode::new(i, kind);

        // === ESTA ES LA CLAVE ===
        // Si NO es el inodo 0 (Root), le ponemos modo 0.
        // Esto le indica al sistema que está "Libre".
        if i > 0 {
            inode.mode = 0; 
        }
        // ========================

        let encoded = bincode::serialize(&inode)?;
        inodes.extend(encoded);
    }

    Ok(inodes)
}