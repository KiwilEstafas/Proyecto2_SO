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

#[cfg(test)]
mod tests {
    use super::*;
    use bincode;

    #[test]
    fn bitmap_size_rounds_up_correctly() {
        let b0 = create_empty_bitmap(0);
        assert_eq!(b0.len(), 0);

        let b1 = create_empty_bitmap(1);
        assert_eq!(b1.len(), 1);

        let b8 = create_empty_bitmap(8);
        assert_eq!(b8.len(), 1);

        let b9 = create_empty_bitmap(9);
        assert_eq!(b9.len(), 2);
    }

    #[test]
    fn inode_table_root_is_directory_and_rest_files() {
        let raw = create_inode_table(4).unwrap();

        let mut cursor = raw.as_slice();

        let root: Inode = bincode::deserialize_from(&mut cursor).unwrap();
        assert_eq!(root.id, 0);
        matches!(root.kind, InodeKind::Directory);

        for expected_id in 1..4 {
            let inode: Inode = bincode::deserialize_from(&mut cursor).unwrap();
            assert_eq!(inode.id, expected_id);
            matches!(inode.kind, InodeKind::File);
        }
    }
}
