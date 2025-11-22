Por si quiere probar que las cosas funcionen, puede hacer esto: 

## PARTE 1: Pruebas del Sistema de Archivos (FUSE)

#### TERMINAL 1 (Se van a ocupar dos)

1.  **Limpieza:**
    ```bash
    fusermount -uz mnt
    rm -rf disco_qr
    ```
2.  **Formateo (MKFS):**
    ```bash
    cargo run --bin mkfs -- --output disco_qr --blocks 400
    ```
3.  **Montaje (MOUNT):**
    ```bash
    cargo run --bin mount -- disco_qr mnt
    ```
    *(La terminal debe quedarse esperando y decir `Inodos ocupados: 1`).*

---

#### TERMINAL 2: Ejecucion de Pruebas

Ejecuta estos comandos uno por uno y verifica el resultado.

**Prueba 1: Estadisticas del Disco (`statfs`)**
Verifica que el sistema reconoce el tamano.
```bash
df -h mnt/
```
✅ **Esperado:** Debe mostrar el tamano total (aprox 50K) y uso bajo (11% o similar).

**Prueba 2: Crear Archivo (`create` / `getattr`)**
```bash
touch mnt/hola.txt
ls -la mnt/
```
✅ **Esperado:**
*   No debe dar error.
*   El `ls` debe mostrar `hola.txt`.
*   En la Terminal 1 (Mount) debe decir: `DEBUG: Archivo creado con Inodo ID 2`.

**Prueba 3: Escribir Contenido (`write`)**
Vamos a escribir texto que se convertira en QR.
```bash
echo "Este es el proyecto QRFS funcionando" > mnt/hola.txt
```
✅ **Esperado:**
*   En la Terminal 1 debe decir: `DEBUG: Escritos 37 bytes...` (o similar).
*   En la carpeta `disco_qr` debe aparecer un nuevo `.png` (el bloque de datos).

**Prueba 4: Leer Contenido (`read`)**
La prueba de fuego. Vamos a leer el QR decodificado.
```bash
cat mnt/hola.txt
```
✅ **Esperado:** Debe imprimir exactamente: `Este es el proyecto QRFS funcionando`.

**Prueba 5: Renombrar (`rename`)**
```bash
mv mnt/hola.txt mnt/final.txt
ls -la mnt/
```
✅ **Esperado:** El archivo `hola.txt` desaparece y aparece `final.txt`. El contenido sigue siendo el mismo (puedes hacerle `cat`).

**Prueba 6: Verificar Atributos (`stat` / `setattr`)**
```bash
stat mnt/final.txt
```
✅ **Esperado:** Debe mostrar fechas validas y el tamano correcto del archivo.

**Prueba 7: Borrar (`unlink`)**
```bash
rm mnt/final.txt
ls -la mnt/
```
✅ **Esperado:** El directorio debe quedar vacio (solo `.` y `..`).

**Prueba 8: Multiples Archivos**
```bash
echo "Archivo 1" > mnt/test1.txt
echo "Archivo 2" > mnt/test2.txt
echo "Archivo 3" > mnt/test3.txt
ls -la mnt/
cat mnt/test2.txt
```
✅ **Esperado:** Todos los archivos aparecen en el listado y se pueden leer correctamente.

**Prueba 9: Desmontar**
Vuelve a la Terminal 1 y presiona `Ctrl+C` para detener el mount. Luego:
```bash
fusermount -u mnt
```

---

## PARTE 2: Pruebas de Verificacion del Filesystem (FSCK)

### Prueba A: Filesystem Limpio

**1. Crear un filesystem nuevo:**
```bash
rm -rf test_fsck
cargo run --bin mkfs -- --output test_fsck --blocks 400
```

**2. Verificar integridad:**
```bash
cargo run --bin fsck -- test_fsck
```

✅ **Esperado:**
```
fsck.qrfs: iniciando verificacion de 'test_fsck'

[1/6] verificando superblock...
  ok: magic=0x51524653, version=1, bloques=400

[2/6] verificando estructura del disco...
  ok: layout consistente

[3/6] verificando bitmap de bloques libres...
  bloques totales: 400
  bloques usados:  23
  bloques libres:  377

[4/6] verificando tabla de inodos...
  inodos cargados: 1
  ok: todos los inodos son validos

[5/6] verificando directorio raiz...
  ok: inodo raiz existe y es valido

[6/6] verificando coherencia bitmap vs inodos...
  ok: bitmap coherente con inodos

========================================
resultado: filesystem consistente
no se encontraron errores
```

---

### Prueba B: Filesystem con Datos

**1. Montar y agregar archivos:**
```bash
cargo run --bin mount -- test_fsck mnt &
sleep 2
echo "contenido 1" > mnt/archivo1.txt
echo "contenido 2" > mnt/archivo2.txt
echo "contenido 3" > mnt/archivo3.txt
fusermount -u mnt
```

**2. Verificar nuevamente:**
```bash
cargo run --bin fsck -- test_fsck
```

✅ **Esperado:**
*   Deberia mostrar mas bloques usados (aprox 26-30)
*   Deberia mostrar mas inodos cargados (4 en total: root + 3 archivos)
*   Mensaje final: "filesystem consistente"

---

### Prueba C: Deteccion de Corrupcion Critica (Superblock)

**1. Corromper el superblock:**
```bash
rm test_fsck/000000.png
```

**2. Intentar verificar:**
```bash
cargo run --bin fsck -- test_fsck
```

❌ **Esperado:**
```
fsck.qrfs: iniciando verificacion de 'test_fsck'

[1/6] verificando superblock...
fsck.qrfs: error: other error: magic number invalido: esperado 0x51524653, encontrado 0x00000000
```

**Codigo de salida:** 1 (error al ejecutar)

---

### Prueba D: Deteccion de Corrupcion de Bloque de Datos

**1. Recrear filesystem limpio:**
```bash
rm -rf test_fsck
cargo run --bin mkfs -- --output test_fsck --blocks 400
```

**2. Agregar datos:**
```bash
cargo run --bin mount -- test_fsck mnt &
sleep 2
echo "datos importantes" > mnt/importante.txt
fusermount -u mnt
```

**3. Listar bloques creados:**
```bash
ls -lh test_fsck/*.png | tail -10
```

**4. Corromper un bloque de datos (elige uno mayor a 000023.png):**
```bash
rm test_fsck/000025.png
```

**5. Verificar:**
```bash
cargo run --bin fsck -- test_fsck
```

⚠️ **Esperado:**
*   El fsck podria reportar error al leer el bloque corrupto
*   Depende de si el bloque borrado estaba asignado a un archivo o no
*   Si estaba asignado, deberia fallar al validar la coherencia

---

### Prueba E: Filesystem Despues de Uso Normal

**1. Recrear y usar normalmente:**
```bash
rm -rf test_fsck
cargo run --bin mkfs -- --output test_fsck --blocks 400
cargo run --bin mount -- test_fsck mnt &
sleep 2

# crear varios archivos
for i in {1..5}; do
  echo "Contenido del archivo $i" > mnt/archivo$i.txt
done

# borrar algunos
rm mnt/archivo2.txt
rm mnt/archivo4.txt

# renombrar otros
mv mnt/archivo1.txt mnt/renombrado.txt

fusermount -u mnt
```

**2. Verificar coherencia:**
```bash
cargo run --bin fsck -- test_fsck
```

✅ **Esperado:**
*   Deberia mostrar 3 inodos cargados (root + archivo3 + archivo5 + renombrado = 4 total)
*   Algunos bloques huerfanos (los de archivo2 y archivo4 borrados)
*   Mensaje: "advertencias menores encontradas" o "filesystem consistente"

**Nota:** Como nuestro `rmdir` no libera bloques en el bitmap, es normal ver bloques huerfanos.

---

### Interpretacion de Resultados de FSCK

**Codigo de Salida:**
- `0` = filesystem consistente, sin errores
- `1` = error al ejecutar (archivo no existe, argumentos invalidos, superblock corrupto)
- `2` = errores criticos encontrados (bloques asignados marcados como libres)

**Tipos de Problemas:**

1. **Errores Criticos** (codigo 2):
   - Bloques asignados a inodos pero marcados como libres en bitmap
   - Superblock con magic number invalido
   - Estructura del disco con solapamientos

2. **Advertencias Menores** (codigo 0):
   - Bloques marcados como usados pero no asignados (huerfanos)
   - Normal despues de borrar archivos si no se implemento liberacion de bloques

3. **Filesystem Limpio** (codigo 0):
   - Sin errores ni advertencias
   - Bitmap coherente con inodos

---

## PARTE 3: Nota Importante sobre Persistencia (Para la defensa)

*   **El sistema es persistente FISICAMENTE:** Los QRs (Inodos y Datos) se guardan en disco y son permanentes.
*   **El sistema es volatil en NOMBRES:** Como usamos `dir_cache` (un HashMap en RAM) para guardar los nombres de los archivos (`hola.txt` -> Inodo 2), **si apagamos el `mount` y lo vuelvemos a encender luego, los archivos seguiran en el disco (los PNGs existen), pero el `ls` saldra vacio** porque la RAM se borro y perdimos la asociacion Nombre-Inodo.

**Como defender esto?**
Si el profesor pregunta, la respuesta es:
*"Por simplicidad academica y tiempo, implementamos el directorio raiz en memoria (RAM). Los datos y la estructura fisica persisten en los QRs, pero el indice de nombres se reinicia al desmontar. Una version 2.0 guardaria el `dir_cache` en un bloque especial del Inodo Raiz."*

**Demostracion del Problema:**
```bash
# crear y montar
cargo run --bin mkfs -- --output demo_persist --blocks 400
cargo run --bin mount -- demo_persist mnt &
sleep 2

# crear archivos
echo "test" > mnt/archivo.txt
ls -la mnt/  # deberia mostrar archivo.txt

# desmontar
fusermount -u mnt

# verificar que los QRs existen
ls -lh demo_persist/*.png | wc -l  # muchos archivos PNG

# remontar
cargo run --bin mount -- demo_persist mnt &
sleep 2

# listar - VACIO!
ls -la mnt/  # solo muestra . y ..

# pero fsck confirma que los datos existen
fusermount -u mnt
cargo run --bin fsck -- demo_persist  # muestra inodos y bloques usados
```

**Por que pasa:**
- Los inodos y bloques de datos SI persisten en los QRs
- El `dir_cache` (que asocia "archivo.txt" -> Inodo 2) esta solo en RAM
- Al remontar, `dir_cache` se inicializa vacio
- Solucion futura: guardar el dir_cache en un bloque especial del disco

---

## PARTE 4: Resumen de Funcionalidades Probadas

### Operaciones FUSE Funcionando:
- ✅ `statfs` - estadisticas del filesystem
- ✅ `create` - crear archivos
- ✅ `getattr` - obtener metadata
- ✅ `write` - escribir datos (genera QRs)
- ✅ `read` - leer datos (decodifica QRs)
- ✅ `rename` - renombrar archivos
- ✅ `unlink/rmdir` - borrar archivos
- ✅ `readdir` - listar directorio
- ✅ `open` - abrir archivos
- ✅ `access` - validar permisos

### Herramientas CLI Funcionando:
- ✅ `mkfs` - formatear disco con QRs
- ✅ `mount` - montar filesystem via FUSE
- ✅ `fsck` - verificar integridad completa

### Sistema de QR Funcionando:
- ✅ Conversion Bytes -> Base64 -> QR -> PNG
- ✅ Conversion PNG -> QR -> Base64 -> Bytes
- ✅ Almacenamiento persistente en disco
- ✅ Lectura robusta con rqrr

### Limitaciones Conocidas:
- ⚠️ Dir_cache volatil (nombres no persisten entre montajes)
- ⚠️ No se liberan bloques del bitmap al borrar archivos
- ⚠️ Solo soporta directorio raiz (no subdirectorios)