Por si quiere probar que las cosas funcionen, puede hacer esto: 

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
    cargo run --bin mount.qrfs -- disco_qr mnt
    ```
    *(La terminal debe quedarse esperando y decir `Inodos ocupados: 1`).*

---

#### Fase B: Ejecución de Pruebas (Terminal 2)

Ejecuta estos comandos uno por uno y verifica el resultado.

**Prueba 1: Estadísticas del Disco (`statfs`)**
Verifica que el sistema reconoce el tamaño.
```bash
df -h mnt/
```
✅ **Esperado:** Debe mostrar el tamaño total (aprox 50K) y uso bajo (1% o similar).

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
Vamos a escribir texto que se convertirá en QR.
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
✅ **Esperado:** Debe mostrar fechas válidas y el tamaño correcto del archivo.

**Prueba 7: Borrar (`unlink`)**
```bash
rm mnt/final.txt
ls -la mnt/
```
✅ **Esperado:** El directorio debe quedar vacío (solo `.` y `..`).

---

### 3. Nota Importante sobre Persistencia (Para la defensa)


*   **El sistema es persistente FÍSICAMENTE:** Los QRs (Inodos y Datos) se guardan en disco y son permanentes.
*   **El sistema es volátil en NOMBRES:** Como usamos `dir_cache` (un HashMap en RAM) para guardar los nombres de los archivos (`hola.txt` -> Inodo 2), **si apagamos el `mount` y lo vuelvemos a encender luego, los archivos seguirán en el disco (los PNGs existen), pero el `ls` saldrá vacío** porque la RAM se borró y perdimos la asociación Nombre-Inodo.

**¿Cómo defender esto?**
Si el profesor pregunta, la respuesta es:
*"Por simplicidad académica y tiempo, implementamos el directorio raíz en memoria (RAM). Los datos y la estructura física persisten en los QRs, pero el índice de nombres se reinicia al desmontar. Una versión 2.0 guardaría el `dir_cache` en un bloque especial del Inodo Raíz."* (Pero si quiere podemos analizar y ver si se cambia, que creo que sería lo ideal)