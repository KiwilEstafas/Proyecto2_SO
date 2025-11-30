use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use qrfs_core::storage::{BlockStorage, QrStorageManager};
use serde::{Deserialize, Serialize}; // Agregamos Serialize para responder JSON
use std::env;
use std::sync::{Arc, Mutex};
use base64::{engine::general_purpose, Engine as _}; 

// Estructura para recibir datos
#[derive(Deserialize)]
struct ScanData {
    block_id: u32,
    content: String,
}

// Estructura para responder errores al celular
#[derive(Serialize)]
struct ResponseMsg {
    status: String,
    message: String,
}

struct AppState {
    storage: Arc<Mutex<QrStorageManager>>,
}

#[get("/")]
async fn index() -> impl Responder {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Lector QRFS</title>
    <script src="https://unpkg.com/html5-qrcode" type="text/javascript"></script>
    <style>
        body { font-family: sans-serif; padding: 20px; text-align: center; background: #f0f2f5; }
        #reader { width: 100%; max-width: 500px; margin: 0 auto; border: 2px solid #333; }
        .input-group { margin: 20px 0; padding: 10px; background: white; border-radius: 8px; }
        input { padding: 10px; font-size: 1.5rem; width: 80px; text-align: center; }
        .status { margin-top: 20px; padding: 10px; border-radius: 5px; font-weight: bold; }
        .success { background-color: #d4edda; color: #155724; }
        .error { background-color: #f8d7da; color: #721c24; }
    </style>
</head>
<body>
    <h2>📷 Lector QRFS</h2>
    
    <div class="input-group">
        <label>Bloque ID a guardar:</label><br>
        <input type="number" id="blockId" value="0">
    </div>

    <div id="reader"></div>
    <div id="result" class="status">Esperando escaneo...</div>

    <script>
        // Configuración del escáner
        let html5QrcodeScanner = new Html5QrcodeScanner(
            "reader", 
            { fps: 10, qrbox: {width: 250, height: 250} },
            /* verbose= */ false
        );
        
        function onScanSuccess(decodedText, decodedResult) {
            // Pausar para procesar
            html5QrcodeScanner.clear();

            let blockId = document.getElementById('blockId').value;
            let resultDiv = document.getElementById('result');
            
            resultDiv.innerText = "⏳ Procesando Bloque " + blockId + "...";
            resultDiv.className = "status";

            // Limpiar el texto (quitar espacios o saltos de línea accidentales)
            let cleanText = decodedText.trim();

            fetch('/upload', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ 
                    block_id: parseInt(blockId), 
                    content: cleanText 
                })
            })
            .then(response => response.json()) // Esperamos respuesta JSON detallada
            .then(data => {
                if (data.status === "ok") {
                    resultDiv.innerText = "✅ " + data.message;
                    resultDiv.className = "status success";
                    
                    // Auto-incrementar ID
                    document.getElementById('blockId').value = parseInt(blockId) + 1;
                    
                    // Reiniciar cámara en 1.5 segundos
                    setTimeout(() => {
                        html5QrcodeScanner.render(onScanSuccess);
                    }, 1500);
                } else {
                    resultDiv.innerText = "❌ Error: " + data.message;
                    resultDiv.className = "status error";
                    // Reiniciar cámara más lento para que el usuario lea el error
                    setTimeout(() => {
                        html5QrcodeScanner.render(onScanSuccess);
                    }, 3000);
                }
            })
            .catch(err => {
                resultDiv.innerText = "❌ Error de Red: " + err;
                resultDiv.className = "status error";
            });
        }

        html5QrcodeScanner.render(onScanSuccess);
    </script>
</body>
</html>
    "#;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/upload")]
async fn upload_block(data: web::Json<ScanData>, state: web::Data<AppState>) -> impl Responder {
    println!(">> Recibido Bloque ID: {}", data.block_id);
    println!(">> Longitud datos: {} caracteres", data.content.len());

    // INTENTO 1: Decodificación Estándar
    let decode_result = general_purpose::STANDARD.decode(&data.content);

    // INTENTO 2: Si falla, intentar modo "URL Safe" (a veces los lectores QR cambian + por -)
    let bytes = match decode_result {
        Ok(b) => b,
        Err(_) => {
            println!("   Warning: Falló decodificación estándar, probando URL_SAFE...");
            match general_purpose::URL_SAFE_NO_PAD.decode(&data.content) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("   Error Base64: {}", e);
                    return HttpResponse::Ok().json(ResponseMsg {
                        status: "error".to_string(),
                        message: format!("QR corrupto o ilegible: {}", e)
                    });
                }
            }
        }
    };

    let storage = state.storage.lock().unwrap();
    
    // Escribir en disco (Esto regenera el PNG)
    match storage.write_block(data.block_id, &bytes) {
        Ok(_) => {
            println!(">> ✅ Bloque {} guardado correctamente.\n", data.block_id);
            HttpResponse::Ok().json(ResponseMsg {
                status: "ok".to_string(),
                message: format!("Bloque {} guardado.", data.block_id)
            })
        },
        Err(e) => {
            eprintln!(">> ❌ Error escribiendo archivo: {}\n", e);
            HttpResponse::Ok().json(ResponseMsg {
                status: "error".to_string(),
                message: format!("Fallo de escritura: {}", e)
            })
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Uso: cargo run --bin server <carpeta_disco_qr>");
        std::process::exit(1);
    }

    let qr_folder = &args[1];
    
    // Asegurar que la carpeta existe
    std::fs::create_dir_all(qr_folder)?;

    let block_size = 128;
    let total_blocks = 400;

    let storage = QrStorageManager::new(qr_folder, block_size, total_blocks);
    let app_state = web::Data::new(AppState {
        storage: Arc::new(Mutex::new(storage)),
    });

    println!("=============================================");
    println!("📡 Servidor Lector QRFS Activo");
    println!("📂 Carpeta destino: {}", qr_folder);
    println!("=============================================");

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .service(index)
            .service(upload_block)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}