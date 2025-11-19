# TODO.md — Plan de Desarrollo QRFS (Rust + FUSE)

## FASE 1 — Infraestructura base del proyecto
- [X] Crear workspace (`qrfs_core` + `qrfs_cli`)
- [X] Definir módulos base (`disk`, `storage`, `fs`, `errors`)
- [X] Crear binarios CLI: `mkfs`, `mount`, `fsck`
- [ ] Integrar crate de FUSE (`fuse3` o `polyfuse`)

## FASE 2 — Definición del formato del disco (Diseño del FS)
- [ ] Elegir tamaño del bloque (`BLOCK_SIZE`)
- [ ] Diseñar Superblock:
  - [ ] Firma mágica ("QRFS")
  - [ ] Versión
  - [ ] Block size
  - [ ] Cantidad total de bloques
  - [ ] Inicio y longitud del mapa de bloques libres
  - [ ] Inicio de tabla de inodos
  - [ ] Cantidad máxima de inodos
  - [ ] Inicio del directorio raíz
- [ ] Diseñar mapa de bloques libres (bitmap)
- [ ] Definir estructura de los Inodes:
  - [ ] Tipo (archivo/directorio)
  - [ ] Tamaño
  - [ ] Bloques asignados
  - [ ] Timestamps
  - [ ] Permisos
- [ ] Definir formato del directorio:
  - [ ] Entradas `<nombre → inodo>`
  - [ ] Tamaño variable o entradas fijas
  - [ ] Entradas especiales `.` y `..` (si aplica)

## FASE 3 — Implementación del almacenamiento (Block Storage)
### QrStorageManager
- [ ] Implementar `new(path, num_blocks)`
- [ ] Implementar `read_block(id)`
- [ ] Implementar `write_block(id, data)`
- [ ] Prevenir lecturas/escrituras fuera de rango
- [ ] Inicializar archivo físico con bloques vacíos

### Modo de pruebas
- [ ] Crear implementación de storage "in-memory" (Vec<u8>) para tests

## FASE 4 — mkfs (crear un filesystem QRFS)
- [ ] Parsear parámetros (`--size`, `--blocks`, `--output`)
- [ ] Crear archivo físico vacío con tamaño indicado
- [ ] Calcular distribución del disco
- [ ] Escribir Superblock
- [ ] Generar y escribir mapa de bloques libres
- [ ] Inicializar tabla de inodos vacía
- [ ] Crear inodo del directorio raíz
- [ ] Validar integridad leyendo superblock

## FASE 5 — Montaje del FS (mount)
- [ ] Abrir archivo QRFS
- [ ] Leer y validar Superblock
- [ ] Cargar bitmap a memoria
- [ ] Cargar tabla de inodos
- [ ] Cargar directorio raíz
- [ ] Construir estructura `QrfsFilesystem` en memoria
- [ ] Implementar capa de abstracción para operar con FUSE

## FASE 6 — Implementación REAL de las operaciones FUSE

### Operaciones obligatorias
- [ ] `getattr` — obtener metadata
- [ ] `create` — crear un archivo vacío
- [ ] `open` — abrir archivo
- [ ] `read` — leer bytes desde bloques
- [ ] `write` — escribir bytes en los bloques del archivo
- [ ] `rename` — renombrar archivo o directorio
- [ ] `rmdir` — borrar directorios (vacíos)
- [ ] `statfs` — estadísticas del FS
- [ ] `fsync` — forzar escritura a disco
- [ ] `access` — validar permisos

### Operaciones opcionales
- [ ] `mkdir` — crear directorio
- [ ] `readdir` — listar contenido de un directorio
- [ ] `opendir` — abrir directorio

## FASE 7 — fsck (verificación del FS)
- [ ] Confirmar validez del superblock
- [ ] Revisar coherencia del bitmap
- [ ] Validar inodos
- [ ] Confirmar existencia del root directory
- [ ] Validar que bloques usados estén asignados correctamente
- [ ] Reportar errores menores
- [ ] Reportar errores críticos

## FASE 8 — Generación de códigos QR

 - [ ] Elegir la librería de generación QR en Rust (qrcode, qrcode-generator, qr_code o similar)
 - [ ] Crear módulo nuevo qr dentro de qrfs_core
 - [ ] Implementar función:
   - [ ] encode_block_to_qr(block_bytes) -> QrImage
 - [ ] Implementar función CLI en qrfs_cli:
   - [ ] qrfs qr <path_archivo> --out ./qr_output/
 - [ ] Leer el archivo desde el FS montado
   - [ ] Obtener lista de bloques del archivo
   - [ ] Convertir cada bloque → QR
 - [ ] Guardar QR como PNG en una carpeta de salida
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
