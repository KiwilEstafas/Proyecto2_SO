# TODO.md — Plan de Desarrollo QRFS (Rust + FUSE)

## FASE 1 — Infraestructura base del proyecto
- [X] Crear workspace (`qrfs_core` + `qrfs_cli`)
- [X] Definir módulos base (`disk`, `storage`, `fs`, `errors`)
- [X] Crear binarios CLI: `mkfs`, `mount`, `fsck`
- [X] Integrar crate de FUSE (`fuse3` o `polyfuse`)

## FASE 2 — Definición del formato del disco (Diseño del FS)
- [X] Elegir tamaño del bloque (`BLOCK_SIZE`)
- [X] Diseñar Superblock:
  - [X] Firma mágica ("QRFS")
  - [X] Versión
  - [X] Block size
  - [X] Cantidad total de bloques
  - [X] Inicio y longitud del mapa de bloques libres
  - [X] Inicio de tabla de inodos
  - [X] Cantidad máxima de inodos
  - [X] Inicio del directorio raíz
- [X] Diseñar mapa de bloques libres (bitmap)[{
	"resource": "/home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs",
	"owner": "rustc",
	"code": {
		"value": "Click for full compiler diagnostic",
		"target": {
			"$mid": 1,
			"path": "/diagnostic message [2]",
			"scheme": "rust-analyzer-diagnostics-view",
			"query": "2",
			"fragment": "file:///home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs"
		}
	},
	"severity": 8,
	"message": "no field `root_dir` on type `&InMemoryBlockStorage`\navailable fields are: `block_size`, `total_blocks`, `data`",
	"source": "rustc",
	"startLineNumber": 174,
	"startColumn": 25,
	"endLineNumber": 174,
	"endColumn": 33,
	"origin": "extHost1"
},{
	"resource": "/home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs",
	"owner": "rustc",
	"code": {
		"value": "Click for full compiler diagnostic",
		"target": {
			"$mid": 1,
			"path": "/diagnostic message [5]",
			"scheme": "rust-analyzer-diagnostics-view",
			"query": "5",
			"fragment": "file:///home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs"
		}
	},
	"severity": 8,
	"message": "no field `root_dir` on type `&storage::InMemoryBlockStorage`\navailable fields are: `block_size`, `total_blocks`, `data`",
	"source": "rustc",
	"startLineNumber": 174,
	"startColumn": 25,
	"endLineNumber": 174,
	"endColumn": 33,
	"origin": "extHost1"
},{
	"resource": "/home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs",
	"owner": "rustc",
	"code": {
		"value": "Click for full compiler diagnostic",
		"target": {
			"$mid": 1,
			"path": "/diagnostic message [8]",
			"scheme": "rust-analyzer-diagnostics-view",
			"query": "8",
			"fragment": "file:///home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs"
		}
	},
	"severity": 8,
	"message": "no field `root_dir` on type `&InMemoryBlockStorage`\navailable fields are: `block_size`, `total_blocks`, `data`",
	"source": "rustc",
	"startLineNumber": 209,
	"startColumn": 25,
	"endLineNumber": 209,
	"endColumn": 33,
	"origin": "extHost1"
},{
	"resource": "/home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs",
	"owner": "rustc",
	"code": {
		"value": "Click for full compiler diagnostic",
		"target": {
			"$mid": 1,
			"path": "/diagnostic message [7]",
			"scheme": "rust-analyzer-diagnostics-view",
			"query": "7",
			"fragment": "file:///home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs"
		}
	},
	"severity": 8,
	"message": "no field `root_dir` on type `&storage::InMemoryBlockStorage`\navailable fields are: `block_size`, `total_blocks`, `data`",
	"source": "rustc",
	"startLineNumber": 209,
	"startColumn": 25,
	"endLineNumber": 209,
	"endColumn": 33,
	"origin": "extHost1"
},{
	"resource": "/home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs",
	"owner": "rustc",
	"code": {
		"value": "Click for full compiler diagnostic",
		"target": {
			"$mid": 1,
			"path": "/diagnostic message [6]",
			"scheme": "rust-analyzer-diagnostics-view",
			"query": "6",
			"fragment": "file:///home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs"
		}
	},
	"severity": 8,
	"message": "the method `detect_grids` exists for struct `PreparedImage<DynamicImage>`, but its trait bounds were not satisfied\nthe following trait bounds were not satisfied:\n`<DynamicImage as GenericImageView>::Pixel = Luma<u8>`\nwhich is required by `DynamicImage: rqrr::prepare::ImageBuffer`",
	"source": "rustc",
	"startLineNumber": 183,
	"startColumn": 29,
	"endLineNumber": 183,
	"endColumn": 41,
	"relatedInformation": [
		{
			"startLineNumber": 71,
			"startColumn": 1,
			"endLineNumber": 71,
			"endColumn": 22,
			"message": "doesn't satisfy `<_ as GenericImageView>::Pixel = Luma<u8>` or `DynamicImage: rqrr::prepare::ImageBuffer`",
			"resource": "/home/estudiante/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/image-0.25.9/src/images/dynimage.rs"
		}
	],
	"origin": "extHost1"
},{
	"resource": "/home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs",
	"owner": "rustc",
	"code": {
		"value": "Click for full compiler diagnostic",
		"target": {
			"$mid": 1,
			"path": "/diagnostic message [3]",
			"scheme": "rust-analyzer-diagnostics-view",
			"query": "3",
			"fragment": "file:///home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs"
		}
	},
	"severity": 8,
	"message": "type mismatch resolving `<DynamicImage as GenericImageView>::Pixel == Luma<u8>`\nexpected struct `Luma<u8>`\n   found struct `Rgba<u8>`\nrequired for `DynamicImage` to implement `rqrr::prepare::ImageBuffer`",
	"source": "rustc",
	"startLineNumber": 181,
	"startColumn": 56,
	"endLineNumber": 181,
	"endColumn": 59,
	"relatedInformation": [
		{
			"startLineNumber": 181,
			"startColumn": 27,
			"endLineNumber": 181,
			"endColumn": 55,
			"message": "required by a bound introduced by this call",
			"resource": "/home/estudiante/Escritorio/Proyecto2_SO/qrfs_core/src/storage.rs"
		},
		{
			"startLineNumber": 180,
			"startColumn": 8,
			"endLineNumber": 180,
			"endColumn": 19,
			"message": "required by a bound in `PreparedImage::<S>::prepare`",
			"resource": "/home/estudiante/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rqrr-0.10.0/src/prepare.rs"
		}
	],
	"origin": "extHost1"
}]
- [X] Definir estructura de los Inodes:
  - [X] Tipo (archivo/directorio)
  - [X] Tamaño
  - [X] Bloques asignados
  - [X] Timestamps
  - [X] Permisos
- [X] Definir formato del directorio:
  - [X] Entradas `<nombre → inodo>`
  - [X] Tamaño variable o entradas fijas
  - [X] Entradas especiales `.` y `..` (si aplica)

## FASE 3 — Implementación del almacenamiento (Block Storage)
### QrStorageManager
- [X] Implementar `new(path, num_blocks)`
- [X] Implementar `read_block(id)`
- [X] Implementar `write_block(id, data)`
- [X] Prevenir lecturas/escrituras fuera de rango
- [X] Inicializar archivo físico con bloques vacíos

### Modo de pruebas
- [X] Crear implementación de storage "in-memory" (Vec<u8>) para tests

## FASE 4 — mkfs (crear un filesystem QRFS)
- [X] Parsear parámetros (`--size`, `--blocks`, `--output`)
- [X] Crear archivo físico vacío con tamaño indicado
- [X] Calcular distribución del disco
- [X] Escribir Superblock
- [X] Inicializar tabla de inodos vacía
- [X] Crear inodo del directorio raíz
- [X] Validar integridad leyendo superblock

## FASE 5 — Montaje del FS (mount)
- [X] Abrir archivo QRFS
- [X] Leer y validar Superblock 
- [X] Cargar bitmap a memoria 
- [X] Cargar tabla de inodos 
- [X] Cargar directorio raíz
- [X] Construir estructura `QrfsFilesystem` en memoria
- [X] Implementar capa de abstracción para operar con FUSE 

## FASE 6 — Implementación REAL de las operaciones FUSE

### Operaciones obligatorias
- [X] `getattr` — obtener metadata
- [X] `create` — crear un archivo vacío
- [X] `open` — abrir archivo
- [X] `read` — leer bytes desde bloques
- [X] `write` — escribir bytes en los bloques del archivo
- [X] `rename` — renombrar archivo o directorio
- [X] `rmdir` — borrar directorios (vacíos) (cREO QUE BORRA TODS)
- [X] `statfs` — estadísticas del FS
- [X] `fsync` — forzar escritura a disco (Se hace automaticamente en write, entonces solo devuelve ok)
- [X] `access` — validar permisos

### Operaciones opcionales
- [ ] `mkdir` — crear directorio
- [X] `readdir` — listar contenido de un directorio
- [ ] `opendir` — abrir directorio

## FASE 7 — fsck (verificación del FS)
- [X] Confirmar validez del superblock
- [X] Revisar coherencia del bitmap
- [X] Validar inodos
- [X] Confirmar existencia del root directory
- [X] Validar que bloques usados estén asignados correctamente
- [X] Reportar errores menores
- [X] Reportar errores críticos

## FASE 8 — Generación de códigos QR

 - [X] Elegir la librería de generación QR en Rust (qrcode, qrcode-generator, qr_code o similar)
 - [ ] Crear módulo nuevo qr dentro de qrfs_core
 - [ ] Implementar función:
   - [X] encode_block_to_qr(block_bytes) -> QrImage
 - [ ] Implementar función CLI en qrfs_cli:
   - [ ] qrfs qr <path_archivo> --out ./qr_output/
 - [X] Leer el archivo desde el FS montado
   - [X] Obtener lista de bloques del archivo
   - [X] Convertir cada bloque → QR
 - [X] Guardar QR como PNG en una carpeta de salida
 - [ ] Validar:
   - [ ] Nombre correlativo (block_0001.png)
   - [ ] Manejo correcto de archivos grandes
   - [ ] Verificación de contenido QR

## FASE 8 — Integración CLI
- [ ] `qrfs mkfs disk.qrfs --size 10MB`
- [ ] `qrfs mount disk.qrfs /mnt/qrfs`
- [ ] `qrfs fsck disk.qrfs`
- [ ] Mensajes de error claros y consistentes
- [ ] Logging básico

## FASE 9 — Pruebas
- [ ] Unit tests: inodos, bitmap, directorios
- [ ] Tests de integración: mkfs + mount + fuse
- [ ] Tests de `read/write`
- [ ] Tests de manejo de errores

## FASE 10 — Pulido
- [ ] Remover warnings
- [ ] Documentación
- [ ] Limpiar codigo y comments
