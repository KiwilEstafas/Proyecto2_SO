use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    inodes: HashMap<u32, Inode>,     // Mapa de inodos cargados en memoria
    bitmap: Vec<u8>,                 // Mapa de bits de bloques usados/libres
    dir_cache: HashMap<String, u32>, // Cache de nombres de archivos a inodo ID para readdir/lookup
}

impl<B: BlockStorage + 'static> QrfsFilesystem<B> {
    pub fn new(storage: Arc<B>) -> Result<Self, crate::errors::QrfsError> {
        // 1. Leer Superblock
        let sb_data = storage.read_block(0)?;
        let superblock: Superblock = bincode::deserialize(&sb_data)
            .map_err(|_| crate::errors::QrfsError::Other("Bloque 0 ilegible".into()))?;

        if !superblock.is_valid() {
            return Err(crate::errors::QrfsError::Other("Firma inválida".into()));
        }

        // 2. Cargar Bitmap (NUEVO) -----------------------------------------
        // Leemos los bloques del mapa de bits. En mkfs usamos 1 solo bloque.
        let mut bitmap = Vec::new();
        for i in 0..superblock.free_map_blocks {
            let data = storage.read_block(superblock.free_map_start + i)?;
            bitmap.extend_from_slice(&data);
        }
        // Recortamos al tamaño exacto de bytes necesarios
        let total_bytes = (superblock.total_blocks as usize + 7) / 8;
        if bitmap.len() > total_bytes {
            bitmap.truncate(total_bytes);
        }
        // ------------------------------------------------------------------

        // 3. Cargar Inodos
        let mut inodes = HashMap::new();
        let mut inode_buffer = Vec::new();

        for i in 0..superblock.inode_table_blocks {
            let data = storage.read_block(superblock.inode_table_start + i)?;
            inode_buffer.extend_from_slice(&data);
        }

        let mut cursor = std::io::Cursor::new(inode_buffer);
        for _ in 0..superblock.inode_count {
            if let Ok(inode) = bincode::deserialize_from::<_, Inode>(&mut cursor) {
                // Solo cargamos inodos válidos (ID 0 es inválido, modo 0 es libre)
                if inode.id == 0 || inode.mode != 0 {
                    inodes.insert(inode.id, inode);
                }
            }
        }

        println!("DEBUG: FS Montado. Inodos ocupados: {}, ...", inodes.len());
        println!(
            "DEBUG: FS Montado. Inodos: {}, Bitmap bytes: {}",
            inodes.len(),
            bitmap.len()
        );

        Ok(Self {
            storage,
            superblock,
            inodes,
            bitmap,
            dir_cache: HashMap::new(),
        })
    }

    pub fn mount(self, mountpoint: &Path) -> Result<(), crate::errors::QrfsError> {
        let options = vec![
            MountOption::RW, // Modo lectura-escritura
            MountOption::FSName("qrfs".to_string()),
        ];

        // Esta función bloquea el programa hasta que desmontes el disco
        fuser::mount2(self, mountpoint, &options)
            .map_err(|e| crate::errors::QrfsError::Other(format!("FUSE Error: {}", e)))?;
        Ok(())
    }

    //-----------------HELPER METHODS PARA MANEJO DE INODOS --------------------
    /// Guarda toda la tabla de inodos de memoria al disco (QRs)
    fn save_inode_table(&self) -> Result<(), crate::errors::QrfsError> {
        // 1. Convertir el HashMap a un vector ordenado por ID
        let mut inodes: Vec<&Inode> = self.inodes.values().collect();
        inodes.sort_by_key(|i| i.id);

        // 2. Serializar todo el vector secuencialmente
        let mut serialized_data = Vec::new();
        for inode in inodes {
            let bytes = bincode::serialize(inode)
                .map_err(|_| crate::errors::QrfsError::Other("Error serializando inodo".into()))?;
            serialized_data.extend_from_slice(&bytes);
        }

        // 3. Escribir en los bloques asignados a la tabla
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

            // Escribir al Storage (esto genera el QR nuevo)
            self.storage.write_block(block_id, &chunk)?;
        }

        Ok(())
    }

    /// Encuentra un ID de inodo libre
    fn find_free_inode_id(&self) -> Option<u32> {
        // Inodo 0 es inválido, inodo 1 es root
        for i in 2..self.superblock.inode_count {
            if !self.inodes.contains_key(&i) {
                return Some(i);
            }
        }
        None
    }
}

impl<B: BlockStorage + 'static> Filesystem for QrfsFilesystem<B> {
    // GETATTR: Obtener metadatos (size, permisos, fecha)
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
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
                blksize: 512,
            };
            reply.attr(&TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    // READDIR: Listar contenido de un directorio (ls)
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

        // Entradas base
        let mut entries = vec![
            (1, FileType::Directory, ".".to_string()),
            (1, FileType::Directory, "..".to_string()),
        ];

        // --- AGREGAR ARCHIVOS DEL CACHE ---
        for (name, &id) in &self.dir_cache {
            // Buscamos el tipo en el inodo real
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
        // ----------------------------------

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // entry.2 es el nombre (String)
            if reply.add(entry.0, (i + 1) as i64, entry.1, &entry.2) {
                break;
            }
        }
        reply.ok();
    }

    // LOOKUP: Buscar archivo por nombre
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        // Solo soportamos búsquedas en la raíz (parent 1)
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        // Convertir nombre a String
        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        // Casos especiales . y ..
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

        // --- BUSCAR EN EL CACHE ---
        if let Some(&inode_id) = self.dir_cache.get(name_str) {
            // Si existe el nombre, buscamos sus datos de inodo
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

        // Si no lo encontramos
        reply.error(ENOENT);
    }
    // Access: Validar permisos de acceso para que el SO permita operaciones
    fn access(&mut self, _req: &Request, ino: u64, _mask: i32, reply: fuser::ReplyEmpty) {
        // Permitimos todo por ahora!!!!
        //En los FS de verdad se deberia chequear permisos RWX contra el uid/gid del proceso
        reply.ok();
    }

    // Statfs: Obtener información del sistema de archivos
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: fuser::ReplyStatfs) {
        let total_blocks = self.superblock.total_blocks as u64;
        let block_size = self.superblock.block_size as u32;

        //contar bloques libres usando el bitmap
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
            free_blocks, //bloques disponibles para usuarios sin  privilegios
            total_inodes,
            free_inodes,
            block_size,
            255, // nombre maximo de archivos en un directorio
            0,
        );
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
        println!("DEBUG: setattr solicitado para inodo {}", ino);

        // Truco: Buscamos el inodo y devolvemos sus datos actuales SIN cambiar nada.
        // Esto engaña a 'touch' haciéndole creer que todo salió bien.

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
                blksize: 512, // <--- Recordar usar 512 aquí también
            };
            reply.attr(&TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        println!("DEBUG: create solicitado en padre {}", parent);
        std::io::stdout().flush().unwrap();

        // 1. Verificar que hay espacio (Inodos libres)
        let new_id = match self.find_free_inode_id() {
            Some(id) => id,
            None => {
                reply.error(libc::ENOSPC); // No space left
                return;
            }
        };

        // 2. Crear el objeto Inodo
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let new_inode = Inode {
            id: new_id,
            kind: InodeKind::File,
            size: 0,
            blocks: Vec::new(),
            mode: mode as u16, // Permisos que pidió el usuario
            created_at: now,
            modified_at: now,
        };

        // 3. Guardar en Memoria
        self.inodes.insert(new_id, new_inode.clone());
        // También actualizar la caché de directorio (nombre -> ID)
        if let Some(filename) = name.to_str() {
            self.dir_cache.insert(filename.to_string(), new_id);
            println!("DEBUG: Nombre '{}' asociado al Inodo {}", filename, new_id);
        }

        // 4. Guardar en Disco (Actualizar QRs de la tabla)
        if let Err(e) = self.save_inode_table() {
            println!("ERROR guardando inodo: {}", e);
            reply.error(libc::EIO);
            return;
        }

        // 5. Responder a FUSE que todo salió bien
        // Nota: FUSE necesita los atributos del archivo recién creado
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

        // El '0' es el generation number y el '0' final es el file handle (fh)
        reply.created(&TTL, &attr, 0, 0, 0);

        println!("DEBUG: Archivo creado con Inodo ID {}", new_id);
        std::io::stdout().flush().unwrap();
    }
}
