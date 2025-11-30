Es un archivo bash para cumplir con la nota en TODO.md y el requsito del profe.

```bash
# Ver ayuda
./qrfs help

# Formatear
./qrfs mkfs --output disco_final --blocks 400

# Verificar
./qrfs fsck disco_final

# Extraer QRs
./qrfs qr disco_final 0 --out ./salida

# Montar
./qrfs mount disco_final mnt
```

Este scrip usa cargo run, por lo que con solo hacer ./qrfs se va a compilar y ejecutar el proyecto. 