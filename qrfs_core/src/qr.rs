// modulo para operaciones de codificacion y decodificacion de bloques a qr

use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, Luma};
use qrcode::QrCode;
use rqrr;

use crate::errors::QrfsError;

/// convierte bytes binarios a una imagen qr
/// 
/// proceso: bytes -> base64 -> qr -> imagen
pub fn encode_block_to_qr(data: &[u8]) -> Result<DynamicImage, QrfsError> {
    // 1. codificar bytes a base64 para que sea texto valido
    let b64_string = general_purpose::STANDARD.encode(data);

    // 2. generar codigo qr desde el string base64
    let code = QrCode::new(b64_string)
        .map_err(|e| QrfsError::Other(format!("error generando qr: {}", e)))?;

    // 3. renderizar a imagen de 200x200 pixeles
    let image = code
        .render::<Luma<u8>>()
        .min_dimensions(200, 200)
        .max_dimensions(200, 200)
        .build();

    // 4. convertir a dynamicimage para mayor flexibilidad
    Ok(DynamicImage::ImageLuma8(image))
}

/// decodifica una imagen qr de vuelta a bytes binarios
/// 
/// proceso: imagen -> qr -> base64 -> bytes
pub fn decode_qr_to_block(img: &DynamicImage) -> Result<Vec<u8>, QrfsError> {
    // 1. convertir imagen a escala de grises
    let img_gray = img.to_luma8();

    // 2. preparar imagen para deteccion qr
    let mut decoder = rqrr::PreparedImage::prepare(img_gray);
    let grids = decoder.detect_grids();

    if grids.is_empty() {
        return Err(QrfsError::Other("no se detecto codigo qr en la imagen".into()));
    }

    // 3. decodificar contenido del qr (devuelve string utf-8)
    let (_meta, content_string) = grids[0]
        .decode()
        .map_err(|e| QrfsError::Other(format!("error decodificando qr: {}", e)))?;

    // 4. decodificar base64 a bytes binarios
    let data = general_purpose::STANDARD
        .decode(content_string)
        .map_err(|e| QrfsError::Other(format!("error decodificando base64: {}", e)))?;

    Ok(data)
}

/// valida que un bloque qr pueda ser decodificado correctamente
/// 
/// retorna el tamaño de los datos decodificados o error
pub fn validate_qr_block(img: &DynamicImage) -> Result<usize, QrfsError> {
    let data = decode_qr_to_block(img)?;
    Ok(data.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let original_data = b"hello qrfs world 12345";
        
        // codificar
        let qr_image = encode_block_to_qr(original_data).unwrap();
        
        // decodificar
        let decoded_data = decode_qr_to_block(&qr_image).unwrap();
        
        // verificar que son iguales
        assert_eq!(original_data.to_vec(), decoded_data);
    }

    #[test]
    fn encode_empty_data() {
        let empty_data = b"";
        let result = encode_block_to_qr(empty_data);
        assert!(result.is_ok());
    }

    #[test]
    fn decode_invalid_image_fails() {
        // crear imagen vacia sin qr
        let empty_img = DynamicImage::new_luma8(200, 200);
        let result = decode_qr_to_block(&empty_img);
        assert!(result.is_err());
    }

    #[test]
    fn validate_qr_block_works() {
        let test_data = b"test validation";
        let qr_image = encode_block_to_qr(test_data).unwrap();
        
        let size = validate_qr_block(&qr_image).unwrap();
        assert_eq!(size, test_data.len());
    }
}