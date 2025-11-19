# TODO.md – Plan de Desarrollo QRFS (Rust)

## FASE 1 — Infraestructura base del proyecto
- [X] Crear estructura del workspace (`qrfs_core` + `qrfs_cli`)
- [X] Definir módulos base (`disk`, `storage`, `fs`, `errors`)
- [X] Crear binarios CLI: `mkfs`, `mount`, `fsck`

## FASE 2 — Definición del formato del disco
- [ ] Definir tamaño del bloque (BLOCK_SIZE)
- [ ] Diseñar estructura final del Superblock
   - [ ] Firma mágica ("QRFS")
   - [ ] Versión
   - [ ] Block size
   - [ ] Cantidad total de bloques
   - [ ] Ubicación del mapa de bloques libres
   - [ ] Ubicación de los inodos
   - [ ] Ubicación del directorio raíz
- [ ] Definir estructura del mapa de bloques libres
- [ ] Definir estructura de los Inodes
- [ ] Definir estructura del directorio raíz

## FASE 3 — Implementación del almacenamiento

### `QrStorageManager` (bloques)
- [ ] Implementar `new(path, num_blocks)` para crear archivo
- [ ] Implementar `read_block(id)`
- [ ] Implementar `write_block(id, data)`
- [ ] Manejar límites y errores (bloque inválido, archivo corrupto)
- [ ] Implementar inicialización de archivo con ceros

### Soporte para testing
- [ ] Crear storage "in-memory" para pruebas unitarias

## FASE 4 — Implementación de mkfs (crear filesystem)
- [ ] Parseo de argumentos (tamaño del disco, archivo destino)
- [ ] Crear archivo físico vacío con tamaño calculado
- [ ] Calcular distribución de bloques
- [ ] Escribir Superblock inicial en el bloque 0
- [ ] Escribir mapa de bloques libres inicial
- [ ] Crear directorio raíz vacío e inodo raíz
- [ ] Validar correcto formateo (leer superblock y comprobar)

## FASE 5 — Implementación del montaje (mount)
- [ ] Abrir archivo de disco
- [ ] Leer y validar el Superblock
   - [ ] Firma
   - [ ] Versión
   - [ ] Tamaño del bloque
- [ ] Cargar mapa de bloques libres a memoria
- [ ] Cargar tabla de inodos
- [ ] Construir estructura en memoria `QrfsFilesystem`
- [ ] Implementar operaciones mínimas del filesystem:
   - [ ] Obtener inodo por ID
   - [ ] Leer contenido de un archivo
   - [ ] Listar entrada de un directorio

(No es necesario implementar escritura completa si el curso no lo exige.)

## FASE 6 — Implementación de fsck (verificación)
- [ ] Leer y validar Superblock
- [ ] Verificar mapa de bloques libres vs bloques usados
- [ ] Verificar consistencia de inodos
- [ ] Verificar existencia del directorio raíz
- [ ] Reportar errores corregibles
- [ ] Reportar errores críticos

## FASE 7 — Mejoras opcionales / pulido
- [ ] Limpiar warnings del compilador, código y comments
- [ ] Agregar documentación
