// modulo para operaciones de validacion de codigos qr

use base64::{engine::general_purpose, Engine as _};
use image::DynamicImage;
use rqrr;

use crate::errors::QrfsError;

// valida que un bloque qr pueda ser decodificado correctamente
// retorna el tamaño de los datos decodificados o error
pub fn validate_qr_block(img: &DynamicImage) -> Result<usize, QrfsError> {
    let img_gray = img.to_luma8();

    let mut decoder = rqrr::PreparedImage::prepare(img_gray);
    let grids = decoder.detect_grids();

    if grids.is_empty() {
        return Err(QrfsError::Other("no se detecto codigo qr en la imagen".into()));
    }

    let (_meta, content_string) = grids[0]
        .decode()
        .map_err(|e| QrfsError::Other(format!("error decodificando qr: {}", e)))?;

    let data_size = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content_string) {
        if let Some(data_str) = parsed.get("data").and_then(|v| v.as_str()) {
            let decoded = general_purpose::STANDARD
                .decode(data_str)
                .map_err(|e| QrfsError::Other(format!("error decodificando base64: {}", e)))?;
            decoded.len()
        } else {
            let decoded = general_purpose::STANDARD
                .decode(&content_string)
                .map_err(|e| QrfsError::Other(format!("error decodificando base64: {}", e)))?;
            decoded.len()
        }
    } else {
        let decoded = general_purpose::STANDARD
            .decode(&content_string)
            .map_err(|e| QrfsError::Other(format!("error decodificando base64: {}", e)))?;
        decoded.len()
    };

    Ok(data_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;
    use qrcode::QrCode;

    #[test]
    fn validate_qr_block_works() {
        let test_data = b"test validation";
        let b64_string = general_purpose::STANDARD.encode(test_data);
        let metadata = format!(r#"{{"block_id":0,"data":"{}"}}"#, b64_string);
        
        let code = QrCode::new(metadata).unwrap();
        let image = code
            .render::<Luma<u8>>()
            .min_dimensions(200, 200)
            .max_dimensions(200, 200)
            .build();
        let qr_image = DynamicImage::ImageLuma8(image);
        
        let size = validate_qr_block(&qr_image).unwrap();
        assert_eq!(size, test_data.len());
    }

    #[test]
    fn validate_qr_block_old_format_works() {
        let test_data = b"test validation old";
        let b64_string = general_purpose::STANDARD.encode(test_data);
        
        let code = QrCode::new(b64_string).unwrap();
        let image = code
            .render::<Luma<u8>>()
            .min_dimensions(200, 200)
            .max_dimensions(200, 200)
            .build();
        let qr_image = DynamicImage::ImageLuma8(image);
        
        let size = validate_qr_block(&qr_image).unwrap();
        assert_eq!(size, test_data.len());
    }

    #[test]
    fn decode_invalid_image_fails() {
        let empty_img = DynamicImage::new_luma8(200, 200);
        let result = validate_qr_block(&empty_img);
        assert!(result.is_err());
    }
}