use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use crate::disk::DirectoryEntry;
use crate::disk::{Inode, InodeKind, BLOCK_SIZE};
use crate::storage::BlockStorage;
use crate::Superblock;

use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;

const TTL: Duration = Duration::from_secs(1);

// implementacion de qrfs que implementa fuser::filesystem
pub struct QrfsFilesystem<B: BlockStorage + 'static> {
    storage: Arc<B>,
    superblock: Superblock,
    inodes: HashMap<u32, Inode>,
    bitmap: Vec<u8>,
    dir_cache: HashMap<String, u32>,
}

impl<B: BlockStorage + 'static> QrfsFilesystem<B> {
    pub fn new(storage: Arc<B>) -> Result<Self, crate::errors::QrfsError> {
        // leer superblock
        let sb_data = storage.read_block(0)?;
        let superblock: Superblock = bincode::deserialize(&sb_data)
            .map_err(|_| crate::errors::QrfsError::Other("bloque 0 ilegible".into()))?;

        if !superblock.is_valid() {
            return Err(crate::errors::QrfsError::Other("firma invalida".into()));
        }

        // cargar bitmap
        let mut bitmap = Vec::new();
        for i in 0..superblock.free_map_blocks {
            let data = storage.read_block(superblock.free_map_start + i)?;
            bitmap.extend_from_slice(&data);
        }
        let total_bytes = (superblock.total_blocks as usize + 7) / 8;
        if bitmap.len() > total_bytes {
            bitmap.truncate(total_bytes);
        }

        // cargar inodos
        let mut inodes = HashMap::new();
        let mut inode_buffer = Vec::new();

        for i in 0..superblock.inode_table_blocks {
            let data = storage.read_block(superblock.inode_table_start + i)?;
            inode_buffer.extend_from_slice(&data);
        }

        let mut cursor = std::io::Cursor::new(inode_buffer);
        for _ in 0..superblock.inode_count {
            if let Ok(inode) = bincode::deserialize_from::<_, Inode>(&mut cursor) {
                if inode.id == 0 || inode.mode != 0 {
                    inodes.insert(inode.id, inode);
                }
            }
        }

        let mut fs = Self {
            storage,
            superblock,
            inodes,
            bitmap,
            dir_cache: HashMap::new(),
        };

        // intentar cargar el directorio raiz del disco
        let root_id = fs.superblock.root_inode;
        println!("debug: cargando directorio raiz (inodo {})...", root_id);

        match fs.load_directory(root_id) {
            Ok(entries) => {
                for entry in entries {
                    if entry.name != "." && entry.name != ".." {
                        fs.dir_cache.insert(entry.name, entry.inode_id);
                    }
                }
                println!(
                    "debug: directorio cargado. {} archivos encontrados.",
                    fs.dir_cache.len()
                );
            }
            Err(e) => {
                println!(
                    "debug: no se pudo cargar directorio (normal si es disco nuevo): {}",
                    e
                );
            }
        }

        Ok(fs)
    }

    pub fn mount(self, mountpoint: &Path) -> Result<(), crate::errors::QrfsError> {
        let options = vec![
            MountOption::RW,
            MountOption::FSName("qrfs".to_string()),
        ];

        fuser::mount2(self, mountpoint, &options)
            .map_err(|e| crate::errors::QrfsError::Other(format!("fuse error: {}", e)))?;
        Ok(())
    }

    // lee los bloques de datos de un inodo (directorio) y devuelve la lista de archivos
    fn load_directory(
        &self,
        inode_id: u32,
    ) -> Result<Vec<DirectoryEntry>, crate::errors::QrfsError> {
        let inode = match self.inodes.get(&inode_id) {
            Some(i) => i,
            None => return Ok(Vec::new()),
        };

        let mut raw_data = Vec::new();
        for &block_id in &inode.blocks {
            let block = self.storage.read_block(block_id)?;
            raw_data.extend_from_slice(&block);
        }

        if inode.size == 0 || raw_data.is_empty() {
            return Ok(Vec::new());
        }

        let valid_data = &raw_data[..inode.size as usize];

        let entries: Vec<DirectoryEntry> = bincode::deserialize(valid_data).map_err(|_| {
            crate::errors::QrfsError::Other("error deserializando directorio".into())
        })?;

        Ok(entries)
    }

    // guarda la lista actual de archivos (dir_cache) en los bloques del inodo raiz
    fn save_root_directory(&mut self) -> Result<(), crate::errors::QrfsError> {
        let root_id = self.superblock.root_inode;

        let mut entries = Vec::new();

        entries.push(DirectoryEntry {
            name: ".".to_string(),
            inode_id: root_id,
            kind: InodeKind::Directory,
        });
        entries.push(DirectoryEntry {
            name: "..".to_string(),
            inode_id: root_id,
            kind: InodeKind::Directory,
        });

        for (name, &id) in &self.dir_cache {
            let kind = if let Some(inode) = self.inodes.get(&id) {
                inode.kind.clone()
            } else {
                InodeKind::File
            };

            entries.push(DirectoryEntry {
                name: name.clone(),
                inode_id: id,
                kind,
            });
        }

        let data = bincode::serialize(&entries)?;
        let total_size = data.len() as u64;

        let mut current_blocks = self.inodes.get(&root_id).unwrap().blocks.clone();

        let block_size = self.superblock.block_size as usize;
        let needed_blocks = (data.len() + block_size - 1) / block_size;

        while current_blocks.len() < needed_blocks {
            if let Some(phys_id) = self.allocate_block() {
                current_blocks.push(phys_id);
            } else {
                return Err(crate::errors::QrfsError::Other(
                    "disco lleno guardando directorio".into(),
                ));
            }
        }

        let mut offset = 0;
        for (_i, &block_id) in current_blocks.iter().enumerate() {
            let mut chunk = vec![0u8; block_size];

            if offset < data.len() {
                let end = std::cmp::min(offset + block_size, data.len());
                let slice = &data[offset..end];
                chunk[..slice.len()].copy_from_slice(slice);
                offset += slice.len();
            }

            self.storage.write_block(block_id, &chunk)?;
        }

        self.save_bitmap()?;

        if let Some(root_inode) = self.inodes.get_mut(&root_id) {
            root_inode.blocks = current_blocks;
            root_inode.size = total_size;
            root_inode.modified_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }

        self.save_inode_table()?;

        Ok(())
    }

    // guarda toda la tabla de inodos de memoria al disco (qrs)
    fn save_inode_table(&self) -> Result<(), crate::errors::QrfsError> {
        let mut serialized_data = Vec::new();

        for id in 0..self.superblock.inode_count {
            let inode_to_write = if let Some(inode) = self.inodes.get(&id) {
                inode.clone()
            } else {
                let mut empty = Inode::new(id, InodeKind::File);
                empty.mode = 0;
                empty
            };

            let bytes = bincode::serialize(&inode_to_write)
                .map_err(|_| crate::errors::QrfsError::Other("error serializando inodo".into()))?;
            serialized_data.extend_from_slice(&bytes);
        }

        let block_size = self.superblock.block_size as usize;
        let start_block = self.superblock.inode_table_start;
        let num_blocks = self.superblock.inode_table_blocks;

        let mut offset = 0;
        for i in 0..num_blocks {
            let block_id = start_block + i;
            let mut chunk = vec![0u8; block_size];

            if offset < serialized_data.len() {
                let end = std::cmp::min(offset + block_size, serialized_data.len());
                let slice = &serialized_data[offset..end];
                chunk[..slice.len()].copy_from_slice(slice);
                offset += slice.len();
            }

            self.storage.write_block(block_id, &chunk)?;
        }

        Ok(())
    }

    // encuentra un id de inodo libre
    fn find_free_inode_id(&self) -> Option<u32> {
        for i in 2..self.superblock.inode_count {
            if !self.inodes.contains_key(&i) {
                return Some(i);
            }
        }
        None
    }

    // guarda el bitmap al disco
    fn save_bitmap(&self) -> Result<(), crate::errors::QrfsError> {
        let block_size = self.superblock.block_size as usize;
        let start_block = self.superblock.free_map_start;
        let num_blocks = self.superblock.free_map_blocks;

        let mut offset = 0;
        for i in 0..num_blocks {
            let block_id = start_block + i;
            let mut chunk = vec![0u8; block_size];

            if offset < self.bitmap.len() {
                let end = std::cmp::min(offset + block_size, self.bitmap.len());
                let slice = &self.bitmap[offset..end];
                chunk[..slice.len()].copy_from_slice(slice);
                offset += slice.len();
            }

            self.storage.write_block(block_id, &chunk)?;
        }

        Ok(())
    }

    // busca un bit libre en el bitmap y lo marca como usado
    fn allocate_block(&mut self) -> Option<u32> {
        let total_blocks = self.superblock.total_blocks as usize;

        for (byte_idx, byte) in self.bitmap.iter_mut().enumerate() {
            if *byte == 0xFF {
                continue;
            }

            for bit_idx in 0..8 {
                let global_id = byte_idx * 8 + bit_idx;

                if global_id < self.superblock.data_block_start as usize {
                    continue;
                }
                if global_id >= total_blocks {
                    return None;
                }

                if (*byte & (1 << bit_idx)) == 0 {
                    *byte |= 1 << bit_idx;
                    return Some(global_id as u32);
                }
            }
        }
        None
    }
}

impl<B: BlockStorage + 'static> Filesystem for QrfsFilesystem<B> {
    // obtener metadatos (size, permisos, fecha)
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        std::io::stdout().flush().unwrap();
        let target = if ino == 1 {
            self.superblock.root_inode
        } else {
            ino as u32
        };

        if let Some(inode) = self.inodes.get(&target) {
            let kind = match inode.kind {
                InodeKind::Directory => FileType::Directory,
                InodeKind::File => FileType::RegularFile,
            };

            let attr = FileAttr {
                ino,
                size: inode.size,
                blocks: 1,
                atime: UNIX_EPOCH,
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind,
                perm: if kind == FileType::Directory {
                    0o755
                } else {
                    0o644
                },
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.attr(&TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    // listar contenido de un directorio (ls)
    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let mut entries = vec![
            (1, FileType::Directory, ".".to_string()),
            (1, FileType::Directory, "..".to_string()),
        ];

        for (name, &id) in &self.dir_cache {
            let kind = if let Some(inode) = self.inodes.get(&id) {
                match inode.kind {
                    InodeKind::Directory => FileType::Directory,
                    InodeKind::File => FileType::RegularFile,
                }
            } else {
                FileType::RegularFile
            };
            entries.push((id as u64, kind, name.clone()));
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(entry.0, (i + 1) as i64, entry.1, &entry.2) {
                break;
            }
        }
        reply.ok();
    }

    // buscar archivo por nombre
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if name_str == "." || name_str == ".." {
            let attr = FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: UNIX_EPOCH,
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.entry(&TTL, &attr, 0);
            return;
        }

        if let Some(&inode_id) = self.dir_cache.get(name_str) {
            if let Some(inode) = self.inodes.get(&inode_id) {
                let kind = match inode.kind {
                    InodeKind::Directory => FileType::Directory,
                    InodeKind::File => FileType::RegularFile,
                };

                let attr = FileAttr {
                    ino: inode_id as u64,
                    size: inode.size,
                    blocks: inode.blocks.len() as u64,
                    atime: UNIX_EPOCH + Duration::from_secs(inode.modified_at),
                    mtime: UNIX_EPOCH + Duration::from_secs(inode.modified_at),
                    ctime: UNIX_EPOCH + Duration::from_secs(inode.created_at),
                    crtime: UNIX_EPOCH + Duration::from_secs(inode.created_at),
                    kind,
                    perm: inode.mode,
                    nlink: 1,
                    uid: 1000,
                    gid: 1000,
                    rdev: 0,
                    flags: 0,
                    blksize: 512,
                };
                reply.entry(&TTL, &attr, 0);
                return;
            }
        }

        reply.error(ENOENT);
    }

    // validar permisos de acceso
    fn access(&mut self, _req: &Request, _ino: u64, _mask: i32, reply: fuser::ReplyEmpty) {
        reply.ok();
    }

    // obtener informacion del sistema de archivos
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: fuser::ReplyStatfs) {
        let total_blocks = self.superblock.total_blocks as u64;
        let block_size = self.superblock.block_size as u32;

        let mut free_blocks = 0;
        for (bite_idx, byte) in self.bitmap.iter().enumerate() {
            for bit in 0..8 {
                let global_bit = bite_idx * 8 + bit;
                if global_bit >= self.superblock.total_blocks as usize {
                    break;
                }
                if (byte & (1 << bit)) == 0 {
                    free_blocks += 1;
                }
            }
        }

        let total_inodes = self.superblock.inode_count as u64;
        let free_inodes = total_inodes - self.inodes.len() as u64;

        reply.statfs(
            total_blocks,
            free_blocks,
            free_blocks,
            total_inodes,
            free_inodes,
            block_size,
            255,
            0,
        );
    }

    fn open(&mut self, _req: &Request, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        let target = if ino == 1 {
            self.superblock.root_inode
        } else {
            ino as u32
        };

        if self.inodes.contains_key(&target) {
            reply.opened(0, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let target = if ino == 1 {
            self.superblock.root_inode
        } else {
            ino as u32
        };

        if let Some(inode) = self.inodes.get(&target) {
            let kind = match inode.kind {
                InodeKind::Directory => FileType::Directory,
                InodeKind::File => FileType::RegularFile,
            };

            let attr = FileAttr {
                ino,
                size: inode.size,
                blocks: inode.blocks.len() as u64,
                atime: UNIX_EPOCH + Duration::from_secs(inode.modified_at),
                mtime: UNIX_EPOCH + Duration::from_secs(inode.modified_at),
                ctime: UNIX_EPOCH + Duration::from_secs(inode.created_at),
                crtime: UNIX_EPOCH + Duration::from_secs(inode.created_at),
                kind,
                perm: inode.mode,
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.attr(&TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        std::io::stdout().flush().unwrap();

        let new_id = match self.find_free_inode_id() {
            Some(id) => id,
            None => {
                reply.error(libc::ENOSPC);
                return;
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let new_inode = Inode {
            id: new_id,
            kind: InodeKind::File,
            size: 0,
            blocks: Vec::new(),
            mode: mode as u16,
            created_at: now,
            modified_at: now,
        };

        self.inodes.insert(new_id, new_inode.clone());
        if let Some(filename) = name.to_str() {
            self.dir_cache.insert(filename.to_string(), new_id);
        }

        if let Err(e) = self.save_root_directory() {
            println!("error: no se pudo persistir el directorio: {}", e);
        }

        if let Err(e) = self.save_inode_table() {
            println!("error guardando inodo: {}", e);
            reply.error(libc::EIO);
            return;
        }

        let attr = FileAttr {
            ino: new_id as u64,
            size: 0,
            blocks: 0,
            atime: UNIX_EPOCH + Duration::from_secs(now),
            mtime: UNIX_EPOCH + Duration::from_secs(now),
            ctime: UNIX_EPOCH + Duration::from_secs(now),
            crtime: UNIX_EPOCH + Duration::from_secs(now),
            kind: FileType::RegularFile,
            perm: mode as u16,
            nlink: 1,
            uid: _req.uid(),
            gid: _req.gid(),
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        reply.created(&TTL, &attr, 0, 0, 0);
        std::io::stdout().flush().unwrap();
    }

    // escribir datos dentro de un archivo
    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        std::io::stdout().flush().unwrap();

        let target = if ino == 1 {
            self.superblock.root_inode
        } else {
            ino as u32
        };
        let block_size = BLOCK_SIZE as u64;

        let offset_in_block = (offset as u64) % block_size;
        let needed_logical_idx = (offset as u64) / block_size;

        let current_blocks = if let Some(inode) = self.inodes.get(&target) {
            inode.blocks.clone()
        } else {
            reply.error(libc::ENOENT);
            return;
        };

        let mut new_block_list = current_blocks;

        while (new_block_list.len() as u64) <= needed_logical_idx {
            if let Some(phys_id) = self.allocate_block() {
                new_block_list.push(phys_id);
                let _ = self.save_bitmap();
            } else {
                reply.error(libc::ENOSPC);
                return;
            }
        }

        let physical_block_id = new_block_list[needed_logical_idx as usize];

        let mut block_data = match self.storage.read_block(physical_block_id) {
            Ok(d) => d,
            Err(_) => vec![0u8; BLOCK_SIZE],
        };

        let end_in_block = std::cmp::min(offset_in_block as usize + data.len(), BLOCK_SIZE);
        let len_to_write = end_in_block - offset_in_block as usize;

        block_data[offset_in_block as usize..end_in_block].copy_from_slice(&data[..len_to_write]);

        if let Err(e) = self.storage.write_block(physical_block_id, &block_data) {
            println!("error escribiendo datos: {}", e);
            reply.error(libc::EIO);
            return;
        }

        if let Some(inode) = self.inodes.get_mut(&target) {
            inode.blocks = new_block_list;

            let new_end = offset as u64 + len_to_write as u64;
            if new_end > inode.size {
                inode.size = new_end;
            }

            let _ = self.save_inode_table();
        }

        reply.written(len_to_write as u32);
        std::io::stdout().flush().unwrap();
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        let target = if ino == 1 {
            self.superblock.root_inode
        } else {
            ino as u32
        };
        let block_size = BLOCK_SIZE as u64;

        if let Some(inode) = self.inodes.get(&target) {
            if offset as u64 >= inode.size {
                reply.data(&[]);
                return;
            }

            let mut data_buffer = Vec::new();
            let mut current_offset = offset as u64;
            let end_offset = std::cmp::min(inode.size, offset as u64 + size as u64);

            while current_offset < end_offset {
                let logical_block_idx = current_offset / block_size;
                let offset_in_block = (current_offset % block_size) as usize;

                let remaining_in_file = end_offset - current_offset;
                let remaining_in_block = (block_size as u64) - (offset_in_block as u64);
                let len_to_read = std::cmp::min(remaining_in_file, remaining_in_block) as usize;

                if (logical_block_idx as usize) < inode.blocks.len() {
                    let phys_id = inode.blocks[logical_block_idx as usize];

                    match self.storage.read_block(phys_id) {
                        Ok(block_data) => {
                            if block_data.len() >= offset_in_block + len_to_read {
                                data_buffer.extend_from_slice(
                                    &block_data[offset_in_block..offset_in_block + len_to_read],
                                );
                            } else {
                                data_buffer.extend(vec![0u8; len_to_read]);
                            }
                        }
                        Err(_) => {
                            reply.error(libc::EIO);
                            return;
                        }
                    }
                } else {
                    data_buffer.extend(vec![0u8; len_to_read]);
                }

                current_offset += len_to_read as u64;
            }

            reply.data(&data_buffer);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn rename(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let name_str = name.to_str().unwrap().to_string();
        let new_name_str = newname.to_str().unwrap().to_string();

        if let Some(inode_id) = self.dir_cache.remove(&name_str) {
            self.dir_cache.insert(new_name_str, inode_id);
            let _ = self.save_root_directory();
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }
        let name_str = name.to_str().unwrap().to_string();

        if let Some(inode_id) = self.dir_cache.remove(&name_str) {
            if let Some(_inode) = self.inodes.remove(&inode_id) {
                // nada que hacer con el inodo
            }
            if let Err(e) = self.save_root_directory() {
                println!("error persistiendo rmdir: {}", e);
            }

            let _ = self.save_inode_table();

            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    // borrar un archivo regular (rm file.txt)
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let name_str = match name.to_str() {
            Some(s) => s.to_string(),
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let inode_id_opt = self.dir_cache.get(&name_str).cloned();

        if let Some(inode_id) = inode_id_opt {
            if let Some(inode) = self.inodes.get(&inode_id) {
                for &block_id in &inode.blocks {
                    let byte_idx = (block_id as usize) / 8;
                    let bit_idx = (block_id as usize) % 8;

                    if byte_idx < self.bitmap.len() {
                        self.bitmap[byte_idx] &= !(1 << bit_idx);
                    }
                }
            } else {
                reply.error(ENOENT);
                return;
            }

            self.inodes.remove(&inode_id);
            self.dir_cache.remove(&name_str);

            if let Err(e) = self.save_root_directory() {
                println!("error al persistir directorio tras borrado: {}", e);
            }

            if let Err(e) = self.save_bitmap() {
                println!("error al guardar bitmap en unlink: {}", e);
                reply.error(libc::EIO);
                return;
            }

            if let Err(e) = self.save_inode_table() {
                println!("error al guardar tabla de inodos en unlink: {}", e);
                reply.error(libc::EIO);
                return;
            }

            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn fsync(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn opendir(&mut self, _req: &Request, ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        let target = if ino == 1 {
            self.superblock.root_inode
        } else {
            ino as u32
        };

        if let Some(inode) = self.inodes.get(&target) {
            match inode.kind {
                InodeKind::Directory => {
                    reply.opened(0, 0);
                }
                InodeKind::File => {
                    reply.error(libc::ENOTDIR);
                }
            }
        } else {
            reply.error(libc::ENOENT);
        }
    }
}
