use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IOError {
    #[error("Failed to read {path:?}")]
    Read { path: PathBuf },

    #[error("Failed to create {path:?}")]
    Create { path: PathBuf },
}

#[derive(Error, Debug)]
#[error("Failed to render {path:?} into {template_name:?} template")]
pub struct RenderError {
    pub path: PathBuf,
    pub template_name: String,
}
