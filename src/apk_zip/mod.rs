pub(in crate::apk_zip) mod zip;
pub(in crate::apk_zip) mod editor;
mod wrap;

pub use wrap::ApkFile;

#[derive(PartialEq)]
pub enum CompressMethod {
    Stored = 0,
    Deflated = 8
}

impl Clone for CompressMethod {
    fn clone(&self) -> Self {
        CompressMethod::convert_from_u16(self.value()).unwrap()
    }
}

impl CompressMethod {
    pub fn convert_from_u16(value: u16) -> Option<CompressMethod> {
        match value {
            0 => Some(CompressMethod::Stored),
            8 => Some(CompressMethod::Deflated),
            _ => None
        }
    }

    pub fn value(&self) -> u16 {
        match self {
            CompressMethod::Stored => 0,
            CompressMethod::Deflated => 8
        }
    }

}

const LOCAL_FILE_HEADER: u32 = 0x4034b50;
const CENTRAL_DIRECTORY_END: u32 = 0x6054b50;
const CENTRAL_DIRECTORY: u32 = 0x2014b50;
