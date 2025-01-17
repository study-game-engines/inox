use std::path::{Path, PathBuf};

use crate::{copy_into_data_folder, ExtensionHandler};
use inox_log::debug_log;
use inox_messenger::MessageHubRc;

const IMAGE_PNG_EXTENSION: &str = "png";
const IMAGE_JPG_EXTENSION: &str = "jpg";
const IMAGE_JPEG_EXTENSION: &str = "jpeg";
const IMAGE_BMP_EXTENSION: &str = "bmp";
const IMAGE_TGA_EXTENSION: &str = "tga";
const IMAGE_DDS_EXTENSION: &str = "dds";
const IMAGE_TIFF_EXTENSION: &str = "tiff";
const IMAGE_GIF_EXTENSION: &str = "bmp";
const IMAGE_ICO_EXTENSION: &str = "ico";

pub struct ImageCompiler {
    message_hub: MessageHubRc,
    data_raw_folder: PathBuf,
    data_folder: PathBuf,
}

impl ImageCompiler {
    pub fn new(message_hub: MessageHubRc, data_raw_folder: &Path, data_folder: &Path) -> Self {
        Self {
            message_hub,
            data_raw_folder: data_raw_folder.to_path_buf(),
            data_folder: data_folder.to_path_buf(),
        }
    }
}

impl ExtensionHandler for ImageCompiler {
    fn on_changed(&mut self, path: &Path) {
        if let Some(ext) = path.extension() {
            let extension = ext.to_str().unwrap().to_string();
            if (extension.as_str() == IMAGE_PNG_EXTENSION
                || extension.as_str() == IMAGE_JPG_EXTENSION
                || extension.as_str() == IMAGE_JPEG_EXTENSION
                || extension.as_str() == IMAGE_BMP_EXTENSION
                || extension.as_str() == IMAGE_TGA_EXTENSION
                || extension.as_str() == IMAGE_TIFF_EXTENSION
                || extension.as_str() == IMAGE_GIF_EXTENSION
                || extension.as_str() == IMAGE_ICO_EXTENSION
                || extension.as_str() == IMAGE_DDS_EXTENSION)
                && copy_into_data_folder(
                    &self.message_hub,
                    path,
                    self.data_raw_folder.as_path(),
                    self.data_folder.as_path(),
                )
            {
                debug_log!("Serializing {:?}", path);
            }
        }
    }
}
