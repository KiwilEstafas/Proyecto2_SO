use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::io::Write; //SOLO PARA DEBUG

use crate::disk::{BlockId, Inode, InodeKind, BLOCK_SIZE};
use crate::errors::QrfsError;
use crate::storage::BlockStorage;
use crate::Superblock;

use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;

const TTL: Duration = Duration::from_secs(1); // Tiempo de vida para atributos de archivo

/// Implementación de QRFS que mas adelante implementara fuser::Filesystem
pub struct QrfsFilesystem<B: BlockStorage + 'static> {
    storage: Arc<B>,
    superblock: Superblock,
    inodes: HashMap<u32, Inode>, // Mapa de inodos cargados en memoria
}

impl<B: BlockStorage + 'static> QrfsFilesystem<B> {
    pub fn new(storage: Arc<B>) -> Result<Self, crate::errors::QrfsError> {
        // 1. LEER SUPERBLOCK (Bloque 0)
        let sb_data = storage.read_block(0)?;
        let superblock: Superblock = bincode::deserialize(&sb_data)
            .map_err(|_| crate::errors::QrfsError::Other("Bloque 0 ilegible o dañado".into()))?;

        if !superblock.is_valid() {
            return Err(crate::errors::QrfsError::Other(
                "Firma del Superblock inválida".into(),
            ));
        }

        // 2. CARGAR TABLA DE INODOS
        // Leemos todos los bloques de la tabla y los deserializamos
        let mut inodes = HashMap::new();
        let mut inode_buffer = Vec::new();

        for i in 0..superblock.inode_table_blocks {
            let data = storage.read_block(superblock.inode_table_start + i)?;
            inode_buffer.extend_from_slice(&data);
        }

        let mut cursor = std::io::Cursor::new(inode_buffer);
        for _ in 0..superblock.inode_count {
            if let Ok(inode) = bincode::deserialize_from::<_, Inode>(&mut cursor) {
                inodes.insert(inode.id, inode);
            }
        }
        println!("DEBUG: Sistema montado. Inodos cargados: {}", inodes.len());

        Ok(Self {
            storage,
            superblock,
            inodes,
        })
    }

    pub fn mount(self, mountpoint: &Path) -> Result<(), crate::errors::QrfsError> {
        let options = vec![
            MountOption::RO, // Read-Only por seguridad inicial
            MountOption::FSName("qrfs".to_string()),
            // MountOption::AutoUnmount,
        ];

        // Esta función bloquea el programa hasta que desmontes el disco
        fuser::mount2(self, mountpoint, &options)
            .map_err(|e| crate::errors::QrfsError::Other(format!("FUSE Error: {}", e)))?;
        Ok(())
    }
}

impl<B: BlockStorage + 'static> Filesystem for QrfsFilesystem<B> {
    // GETATTR: Obtener metadatos (size, permisos, fecha)
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        println!("DEBUG: getattr inodo {}", ino);
        std::io::stdout().flush().unwrap();
        // Mapeo: FUSE usa ino=1 para Root. Nuestro disco usa superblock.root_inode (0)
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
                blksize: BLOCK_SIZE as u32,
            };
            reply.attr(&TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    // READDIR: Listar archivos (ls)
    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            // Solo soportamos listar root por ahora
            reply.error(ENOENT);
            return;
        }

        // Entradas dummy (más adelante leeremos el contenido real del directorio)
        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }

    // LOOKUP: Buscar archivo por nombre
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        // Si preguntan por "." o "..", devolvemos el mismo inodo raíz
        if parent == 1 && (name.to_str() == Some(".") || name.to_str() == Some("..")) {
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
                blksize: 256,
            };
            // TTL de 1 seg y generación 0
            reply.entry(&TTL, &attr, 0);
            return;
        }

        // Para todo lo demás, NO EXISTE (por ahora)
        reply.error(ENOENT);
    }
}
