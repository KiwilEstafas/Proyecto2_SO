use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use qrfs_core::storage::{BlockStorage, QrStorageManager};
use serde::{Deserialize, Serialize}; // Agregamos Serialize para responder JSON
use std::env;
use std::sync::{Arc, Mutex};
use base64::{engine::general_purpose, Engine as _}; 
use serde_json;

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
    <title>lector qrfs - modo manual</title>
    <script src="https://unpkg.com/html5-qrcode" type="text/javascript"></script>
    <style>
        body { font-family: sans-serif; padding: 20px; text-align: center; background: #f0f2f5; }
        #reader { width: 100%; max-width: 500px; margin: 0 auto; border: 2px solid #333; }
        .input-group { margin: 20px 0; padding: 10px; background: white; border-radius: 8px; }
        input[type="number"] { padding: 10px; font-size: 1.5rem; width: 80px; text-align: center; }
        textarea { 
            width: 90%; 
            min-height: 150px; 
            padding: 10px; 
            font-family: monospace; 
            font-size: 0.9rem;
            border: 2px solid #ddd;
            border-radius: 5px;
            margin: 10px 0;
        }
        button {
            background: #4CAF50;
            color: white;
            padding: 12px 24px;
            border: none;
            border-radius: 5px;
            font-size: 1rem;
            cursor: pointer;
            margin: 5px;
        }
        button:hover { background: #45a049; }
        button.secondary { background: #2196F3; }
        button.secondary:hover { background: #0b7dda; }
        .status { margin-top: 20px; padding: 10px; border-radius: 5px; font-weight: bold; }
        .success { background-color: #d4edda; color: #155724; }
        .error { background-color: #f8d7da; color: #721c24; }
        .mode-selector {
            background: white;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 20px;
        }
        .hidden { display: none; }
        .preview-box {
            background: white;
            border: 2px solid #4CAF50;
            border-radius: 8px;
            padding: 15px;
            margin: 20px auto;
            max-width: 600px;
            text-align: left;
        }
        .preview-box h3 {
            margin-top: 0;
            color: #4CAF50;
        }
        .preview-content {
            background: #f5f5f5;
            padding: 10px;
            border-radius: 5px;
            font-family: monospace;
            word-wrap: break-word;
            max-height: 200px;
            overflow-y: auto;
        }
        .metadata {
            background: #e3f2fd;
            padding: 8px;
            border-radius: 5px;
            margin: 10px 0;
            font-size: 0.9rem;
        }
    </style>
</head>
<body>
    <h2>lector qrfs</h2>
    
    <div class="mode-selector">
        <button onclick="showMode('camera')" class="secondary">usar camara</button>
        <button onclick="showMode('manual')">modo manual (pegar texto)</button>
    </div>

    <div class="input-group">
        <label>bloque id a guardar:</label><br>
        <input type="number" id="blockId" value="0">
    </div>

    <div id="cameraMode">
        <div id="reader"></div>
    </div>

    <div id="manualMode" class="hidden">
        <div class="input-group">
            <h3>modo manual</h3>
            <p>escanea el qr con tu celular y pega el contenido aqui:</p>
            <textarea id="qrContent" placeholder='pega aqui el contenido del qr, ejemplo:
{"block_id":0,"data":"SGVsbG8gV29ybGQ="}

o directamente el base64:
SGVsbG8gV29ybGQ='></textarea>
            <br>
            <button onclick="uploadManual()">enviar bloque</button>
        </div>
    </div>

    <div id="result" class="status">esperando escaneo...</div>

    <div id="previewBox" class="preview-box hidden">
        <h3>contenido decodificado del bloque <span id="previewBlockId"></span></h3>
        <div class="metadata">
            <strong>tamanio:</strong> <span id="previewSize"></span> bytes<br>
            <strong>tipo:</strong> <span id="previewType"></span>
        </div>
        <div class="preview-content" id="previewContent"></div>
    </div>

    <script>
        let html5QrcodeScanner = null;
        let currentMode = 'camera';

        function showMode(mode) {
            currentMode = mode;
            if (mode === 'camera') {
                document.getElementById('cameraMode').classList.remove('hidden');
                document.getElementById('manualMode').classList.add('hidden');
                if (!html5QrcodeScanner) {
                    initCamera();
                }
            } else {
                document.getElementById('cameraMode').classList.add('hidden');
                document.getElementById('manualMode').classList.remove('hidden');
                if (html5QrcodeScanner) {
                    html5QrcodeScanner.clear();
                }
            }
        }

        function initCamera() {
            html5QrcodeScanner = new Html5QrcodeScanner(
                "reader", 
                { fps: 10, qrbox: {width: 250, height: 250} },
                false
            );
            html5QrcodeScanner.render(onScanSuccess);
        }

        function decodeAndPreview(content, blockId) {
            try {
                // intentar parsear como json
                const parsed = JSON.parse(content);
                if (parsed.data) {
                    // tiene metadata
                    const decoded = atob(parsed.data);
                    showPreview(blockId, decoded, 'datos del filesystem');
                    return;
                }
            } catch (e) {
                // no es json, intentar decodificar directo
                try {
                    const decoded = atob(content);
                    showPreview(blockId, decoded, 'base64 directo');
                    return;
                } catch (e2) {
                    showPreview(blockId, content, 'texto plano');
                }
            }
        }

        function showPreview(blockId, content, type) {
            document.getElementById('previewBlockId').textContent = blockId;
            document.getElementById('previewSize').textContent = content.length;
            
            // detectar tipo de bloque por id
            let blockType = 'datos desconocidos';
            if (blockId == 0) {
                blockType = 'superblock (metadata del fs)';
            } else if (blockId >= 1 && blockId < 2) {
                blockType = 'bitmap (mapa de bloques libres)';
            } else if (blockId >= 2 && blockId < 10) {
                blockType = 'tabla de inodos';
            } else {
                blockType = 'datos de archivo';
            }
            
            document.getElementById('previewType').textContent = blockType;
            
            // mostrar contenido apropiado segun tipo
            let preview = '';
            
            // si es mayormente bytes nulos, mostrar resumen
            const nullCount = (content.match(/\0/g) || []).length;
            const printableCount = content.replace(/[^\x20-\x7E]/g, '').length;
            
            if (nullCount > content.length * 0.8) {
                // mas del 80% son nulos
                preview = '[bloque de metadata del filesystem]\n\n';
                preview += 'bytes totales: ' + content.length + '\n';
                preview += 'bytes nulos: ' + nullCount + '\n';
                preview += 'bytes con datos: ' + (content.length - nullCount);
            } else if (printableCount > content.length * 0.3) {
                // tiene texto legible
                const printable = content.replace(/[^\x20-\x7E\n\r\t]/g, '.');
                preview = printable.substring(0, 500);
                if (content.length > 500) {
                    preview += '\n\n... (truncado, total: ' + content.length + ' bytes)';
                }
            } else {
                // mostrar hex dump estilo
                preview = '[contenido binario]\n\n';
                const bytes = [];
                for (let i = 0; i < Math.min(64, content.length); i++) {
                    const byte = content.charCodeAt(i).toString(16).padStart(2, '0');
                    bytes.push(byte);
                    if ((i + 1) % 16 === 0) bytes.push('\n');
                }
                preview += bytes.join(' ');
                if (content.length > 64) {
                    preview += '\n\n... (mostrando primeros 64 de ' + content.length + ' bytes)';
                }
            }
            
            // escapar html
            preview = preview.replace(/&/g, '&amp;')
                        .replace(/</g, '&lt;')
                        .replace(/>/g, '&gt;');
            
            document.getElementById('previewContent').innerHTML = preview;
            document.getElementById('previewBox').classList.remove('hidden');
        }
        
        function onScanSuccess(decodedText, decodedResult) {
            html5QrcodeScanner.clear();

            let blockId = document.getElementById('blockId').value;
            let resultDiv = document.getElementById('result');
            
            resultDiv.innerText = "procesando bloque " + blockId + "...";
            resultDiv.className = "status";

            let cleanText = decodedText.trim();

            // mostrar preview antes de enviar
            decodeAndPreview(cleanText, blockId);

            fetch('/upload', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ 
                    block_id: parseInt(blockId), 
                    content: cleanText 
                })
            })
            .then(response => response.json())
            .then(data => {
                if (data.status === "ok") {
                    resultDiv.innerText = "✓ " + data.message;
                    resultDiv.className = "status success";
                    
                    document.getElementById('blockId').value = parseInt(blockId) + 1;
                    
                    setTimeout(() => {
                        html5QrcodeScanner.render(onScanSuccess);
                    }, 1500);
                } else {
                    resultDiv.innerText = "✗ error: " + data.message;
                    resultDiv.className = "status error";
                    setTimeout(() => {
                        html5QrcodeScanner.render(onScanSuccess);
                    }, 3000);
                }
            })
            .catch(err => {
                resultDiv.innerText = "✗ error de red: " + err;
                resultDiv.className = "status error";
            });
        }

        async function uploadManual() {
            let blockId = document.getElementById('blockId').value;
            let content = document.getElementById('qrContent').value.trim();
            let resultDiv = document.getElementById('result');

            if (!content) {
                resultDiv.innerText = "✗ debes pegar el contenido del qr";
                resultDiv.className = "status error";
                return;
            }

            resultDiv.innerText = "enviando bloque " + blockId + "...";
            resultDiv.className = "status";

            // mostrar preview antes de enviar
            decodeAndPreview(content, blockId);

            try {
                const response = await fetch('/upload', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ 
                        block_id: parseInt(blockId), 
                        content: content 
                    })
                });

                const data = await response.json();

                if (data.status === "ok") {
                    resultDiv.innerText = "✓ " + data.message;
                    resultDiv.className = "status success";
                    
                    document.getElementById('blockId').value = parseInt(blockId) + 1;
                    document.getElementById('qrContent').value = '';
                    document.getElementById('qrContent').focus();
                } else {
                    resultDiv.innerText = "✗ error: " + data.message;
                    resultDiv.className = "status error";
                }
            } catch (err) {
                resultDiv.innerText = "✗ error de red: " + err;
                resultDiv.className = "status error";
            }
        }

        // iniciar en modo manual por defecto
        showMode('manual');
    </script>
</body>
</html>
    "#;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/upload")]
async fn upload_block(data: web::Json<ScanData>, state: web::Data<AppState>) -> impl Responder {
    println!(">> recibido bloque id: {}", data.block_id);
    println!(">> longitud datos: {} caracteres", data.content.len());

    // intentar parsear como json con metadata primero
    let bytes = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data.content) {
        // tiene metadata con block_id y data
        if let Some(data_str) = parsed.get("data").and_then(|v| v.as_str()) {
            println!("   formato: json con metadata");
            match general_purpose::STANDARD.decode(data_str) {
                Ok(b) => b,
                Err(_) => {
                    match general_purpose::URL_SAFE_NO_PAD.decode(data_str) {
                        Ok(b) => b,
                        Err(e) => {
                            eprintln!("   error base64: {}", e);
                            return HttpResponse::Ok().json(ResponseMsg {
                                status: "error".to_string(),
                                message: format!("qr corrupto o ilegible: {}", e)
                            });
                        }
                    }
                }
            }
        } else {
            eprintln!("   error: json sin campo 'data'");
            return HttpResponse::Ok().json(ResponseMsg {
                status: "error".to_string(),
                message: "json invalido: falta campo 'data'".to_string()
            });
        }
    } else {
        // no es json, intentar decodificar directo como base64
        println!("   formato: base64 directo");
        match general_purpose::STANDARD.decode(&data.content) {
            Ok(b) => b,
            Err(_) => {
                match general_purpose::URL_SAFE_NO_PAD.decode(&data.content) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("   error base64: {}", e);
                        return HttpResponse::Ok().json(ResponseMsg {
                            status: "error".to_string(),
                            message: format!("qr corrupto o ilegible: {}", e)
                        });
                    }
                }
            }
        }
    };

    let storage = state.storage.lock().unwrap();
    
    match storage.write_block(data.block_id, &bytes) {
        Ok(_) => {
            println!(">> bloque {} guardado correctamente.\n", data.block_id);
            HttpResponse::Ok().json(ResponseMsg {
                status: "ok".to_string(),
                message: format!("bloque {} guardado.", data.block_id)
            })
        },
        Err(e) => {
            eprintln!(">> error escribiendo archivo: {}\n", e);
            HttpResponse::Ok().json(ResponseMsg {
                status: "error".to_string(),
                message: format!("fallo de escritura: {}", e)
            })
        }
    }
}

#[get("/scanner")]
async fn scanner_page() -> impl Responder {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta charset="UTF-8">
    <title>escaner qrfs - modo masivo</title>
    <script src="https://unpkg.com/html5-qrcode" type="text/javascript"></script>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: system-ui, sans-serif; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
            color: white;
        }
        .container {
            max-width: 600px;
            margin: 0 auto;
        }
        h1 {
            text-align: center;
            margin-bottom: 20px;
            font-size: 1.8rem;
            text-shadow: 2px 2px 4px rgba(0,0,0,0.3);
        }
        .stats {
            background: rgba(255,255,255,0.2);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 20px;
            margin-bottom: 20px;
        }
        .stat-row {
            display: flex;
            justify-content: space-between;
            margin: 10px 0;
            font-size: 1.1rem;
        }
        .stat-value {
            font-weight: bold;
            color: #ffd700;
        }
        #reader {
            border-radius: 15px;
            overflow: hidden;
            box-shadow: 0 10px 30px rgba(0,0,0,0.3);
            background: white;
        }
        .log {
            background: rgba(0,0,0,0.4);
            border-radius: 10px;
            padding: 15px;
            margin-top: 20px;
            max-height: 200px;
            overflow-y: auto;
            font-family: monospace;
            font-size: 0.9rem;
        }
        .log-entry {
            margin: 5px 0;
            padding: 5px;
            border-left: 3px solid #4ade80;
            padding-left: 10px;
        }
        .log-entry.error {
            border-left-color: #ef4444;
        }
        .controls {
            text-align: center;
            margin-top: 20px;
        }
        button {
            background: #4ade80;
            color: black;
            border: none;
            padding: 15px 30px;
            border-radius: 10px;
            font-size: 1.1rem;
            font-weight: bold;
            cursor: pointer;
            box-shadow: 0 4px 15px rgba(74, 222, 128, 0.4);
            transition: all 0.3s;
        }
        button:hover {
            transform: translateY(-2px);
            box-shadow: 0 6px 20px rgba(74, 222, 128, 0.6);
        }
        button:disabled {
            background: #6b7280;
            cursor: not-allowed;
            box-shadow: none;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>escaner qrfs - modo masivo</h1>
        
        <div class="stats">
            <div class="stat-row">
                <span>bloques escaneados:</span>
                <span class="stat-value" id="scannedCount">0</span>
            </div>
            <div class="stat-row">
                <span>errores:</span>
                <span class="stat-value" id="errorCount">0</span>
            </div>
            <div class="stat-row">
                <span>ultimo bloque:</span>
                <span class="stat-value" id="lastBlock">-</span>
            </div>
        </div>

        <div id="reader"></div>
        
        <div class="controls">
            <button id="toggleBtn" onclick="toggleScanning()">pausar</button>
        </div>

        <div class="log" id="log"></div>
    </div>

    <script>
        // forzar permisos de camara en http (desarrollo)
        if (navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
            console.log('camara disponible');
        } else {
            alert('tu navegador no soporta acceso a camara o necesitas https');
        }

        let scannedCount = 0;
        let errorCount = 0;
        let isScanning = true;
        let scannedBlocks = new Set();
        let html5QrcodeScanner;

        function addLog(message, isError = false) {
            const log = document.getElementById('log');
            const entry = document.createElement('div');
            entry.className = 'log-entry' + (isError ? ' error' : '');
            const timestamp = new Date().toLocaleTimeString();
            entry.textContent = `[${timestamp}] ${message}`;
            log.insertBefore(entry, log.firstChild);
            
            if (log.children.length > 20) {
                log.removeChild(log.lastChild);
            }
        }

        function updateStats(blockId = null) {
            document.getElementById('scannedCount').textContent = scannedCount;
            document.getElementById('errorCount').textContent = errorCount;
            if (blockId !== null) {
                document.getElementById('lastBlock').textContent = blockId;
            }
        }

        function toggleScanning() {
            const btn = document.getElementById('toggleBtn');
            isScanning = !isScanning;
            
            if (isScanning) {
                btn.textContent = 'pausar';
                html5QrcodeScanner.resume();
                addLog('escaner reanudado');
            } else {
                btn.textContent = 'reanudar';
                html5QrcodeScanner.pause();
                addLog('escaner pausado');
            }
        }

        async function onScanSuccess(decodedText) {
            if (!isScanning) return;

            try {
                const response = await fetch('/upload_auto', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ content: decodedText })
                });

                const data = await response.json();
                
                if (data.status === "ok") {
                    const blockId = data.block_id;
                    
                    if (!scannedBlocks.has(blockId)) {
                        scannedBlocks.add(blockId);
                        scannedCount++;
                        addLog(`bloque ${blockId} guardado correctamente`);
                        updateStats(blockId);
                    } else {
                        addLog(`bloque ${blockId} ya escaneado (omitido)`, false);
                    }
                } else {
                    errorCount++;
                    addLog(`error: ${data.message}`, true);
                    updateStats();
                }
            } catch (err) {
                errorCount++;
                addLog(`error de red: ${err.message}`, true);
                updateStats();
            }
        }

        html5QrcodeScanner = new Html5QrcodeScanner(
            "reader",
            { 
                fps: 10,
                qrbox: { width: 250, height: 250 },
                aspectRatio: 1.0
            },
            false
        );

        html5QrcodeScanner.render(onScanSuccess);
        addLog('escaner iniciado - apunta a los codigos qr');
    </script>
</body>
</html>
    "#;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[derive(Deserialize)]
struct AutoScanData {
    content: String,
}

#[derive(Serialize)]
struct AutoScanResponse {
    status: String,
    message: String,
    block_id: u32,
}

#[post("/upload_auto")]
async fn upload_auto(data: web::Json<AutoScanData>, state: web::Data<AppState>) -> impl Responder {
    println!(">> recibido qr para analisis automatico");
    
    // intentar parsear como json con metadata
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data.content) {
        if let (Some(block_id), Some(data_str)) = (
            parsed.get("block_id").and_then(|v| v.as_u64()),
            parsed.get("data").and_then(|v| v.as_str())
        ) {
            // qr con metadata
            let bytes = match general_purpose::STANDARD.decode(data_str) {
                Ok(b) => b,
                Err(e) => {
                    return HttpResponse::Ok().json(AutoScanResponse {
                        status: "error".to_string(),
                        message: format!("error decodificando base64: {}", e),
                        block_id: 0,
                    });
                }
            };
            
            let storage = state.storage.lock().unwrap();
            
            match storage.write_block(block_id as u32, &bytes) {
                Ok(_) => {
                    println!(">> bloque {} guardado correctamente", block_id);
                    return HttpResponse::Ok().json(AutoScanResponse {
                        status: "ok".to_string(),
                        message: format!("bloque {} guardado", block_id),
                        block_id: block_id as u32,
                    });
                },
                Err(e) => {
                    return HttpResponse::Ok().json(AutoScanResponse {
                        status: "error".to_string(),
                        message: format!("error escribiendo: {}", e),
                        block_id: 0,
                    });
                }
            }
        }
    }
    
    // fallback: si no tiene metadata, usar modo secuencial
    let bytes = match general_purpose::STANDARD.decode(&data.content) {
        Ok(b) => b,
        Err(_) => {
            match general_purpose::URL_SAFE_NO_PAD.decode(&data.content) {
                Ok(b) => b,
                Err(e) => {
                    return HttpResponse::Ok().json(AutoScanResponse {
                        status: "error".to_string(),
                        message: format!("qr corrupto: {}", e),
                        block_id: 0,
                    });
                }
            }
        }
    };
    
    let storage = state.storage.lock().unwrap();
    
    for block_id in 0..storage.total_blocks() {
        let path_str = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
        let path = format!("{}/{:06}.png", path_str, block_id);
        
        if !std::path::Path::new(&path).exists() {
            match storage.write_block(block_id, &bytes) {
                Ok(_) => {
                    println!(">> bloque {} guardado automaticamente", block_id);
                    return HttpResponse::Ok().json(AutoScanResponse {
                        status: "ok".to_string(),
                        message: format!("bloque {} guardado", block_id),
                        block_id,
                    });
                },
                Err(e) => {
                    return HttpResponse::Ok().json(AutoScanResponse {
                        status: "error".to_string(),
                        message: format!("error escribiendo: {}", e),
                        block_id: 0,
                    });
                }
            }
        }
    }
    
    HttpResponse::Ok().json(AutoScanResponse {
        status: "error".to_string(),
        message: "no hay bloques disponibles".to_string(),
        block_id: 0,
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("uso: cargo run --bin server <carpeta_disco_qr>");
        std::process::exit(1);
    }

    let qr_folder = &args[1];
    
    std::fs::create_dir_all(qr_folder)?;

    let block_size = 128;
    let total_blocks = 400;

    let storage = QrStorageManager::new(qr_folder, block_size, total_blocks);
    let app_state = web::Data::new(AppState {
        storage: Arc::new(Mutex::new(storage)),
    });

    println!("=============================================");
    println!("servidor lector qrfs activo");
    println!("carpeta destino: {}", qr_folder);
    println!("=============================================");
    println!();
    println!("modos disponibles:");
    println!("  - modo manual:    http://IP:8080/");
    println!("  - modo escaneo:   http://IP:8080/scanner");
    println!();

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .service(index)
            .service(scanner_page)      // nueva ruta
            .service(upload_block)
            .service(upload_auto)       // nuevo endpoint
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}