use crate::disk::*;
use crate::errors::QrfsError;

/// genera un vector de bytes representando el bitmap
pub fn create_empty_bitmap(total_blocks: u32) -> Vec<u8> {
    let bytes = (total_blocks as usize + 7) / 8;
    vec![0u8; bytes]
}

/// serializa superblock a bytes
pub fn serialize_superblock(sb: &Superblock) -> Result<Vec<u8>, QrfsError> {
    let encoded = bincode::serialize(sb)?;
    Ok(encoded)
}

/// serializa la tabla de inodos inicial
pub fn create_inode_table(count: u32) -> Result<Vec<u8>, QrfsError> {
    let mut inodes = Vec::new();

    for i in 0..count {
        let kind = if i == 0 {
            InodeKind::Directory
        } else {
            InodeKind::File
        };

        let inode = Inode::new(i, kind);
        let encoded = bincode::serialize(&inode)?;
        inodes.extend(encoded);
    }

    Ok(inodes)
}
