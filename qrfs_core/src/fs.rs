use std::path::Path;
use std::sync::Arc;

use crate::errors::QrfsError;
use crate::storage::BlockStorage;

/// Implementación de QRFS que más adelante implementará fuser::Filesystem
pub struct QrfsFilesystem<B: BlockStorage + 'static> {
    storage: Arc<B>,
}

impl<B: BlockStorage + 'static> QrfsFilesystem<B> {
    pub fn new(storage: Arc<B>) -> Self {
        Self { storage }
    }

    /// En el futuro aquí se llamará a fuser::mount
    pub fn mount(&self, _mountpoint: &Path) -> Result<(), QrfsError> {
        Err(QrfsError::Unimplemented(
            "FUSE mount not implemented yet".into(),
        ))
    }
}
