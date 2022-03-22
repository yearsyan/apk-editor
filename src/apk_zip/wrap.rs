use std::error::Error;
use std::io::{Read, Write};
use crate::apk_zip::zip::{ZipFile, ZipFormatError};
use crate::apk_zip::editor::ZipEditor;
use crate::apk_zip::CompressMethod;

pub struct ApkFile<'a> {
    data: &'a Vec<u8>,
    zip: ZipFile<'a>,
    editor: ZipEditor,
    dex_count: usize
}

impl<'a> ApkFile<'a> {

    pub fn from(data: &'a Vec<u8>) -> Result<ApkFile<'a>, ZipFormatError> {
        let zip = ZipFile::from(data)?;
        let editor = ZipEditor::from(&zip);
        let mut dex_count = 0;
        for (name, index) in &zip.file_name_map {
            if name.starts_with("classes") && name.ends_with(".dex") {
                dex_count += 1;
            }
        }
        Ok(ApkFile {
            data,
            zip,
            editor,
            dex_count
        })
    }


    pub fn add_dex<T: AsRef<[u8]>>(&mut self, data: T) {
        let mut file_name = String::from("classes");
        file_name.push_str(self.dex_count.clone().to_string().as_str());
        self.dex_count += 1;
        file_name.push_str(".dex");
        self.editor.append_file(Vec::from(data.as_ref()), file_name, CompressMethod::Deflated);
    }

    pub fn get_manifest(&self) -> Vec<u8> {
        self.zip.get_uncompress_data("AndroidManifest.xml").unwrap()
    }

    pub fn set_manifest<T: AsRef<[u8]>>(&mut self, data: T) {
        self.editor.edit_file(&self.zip, "AndroidManifest.xml", Vec::from(data.as_ref()));
    }

    pub fn add_assets<T: AsRef<[u8]>>(&mut self, name: &str, data: T) {
        let mut path = String::from("assets/");
        path.push_str(name);
        self.editor.append_file(Vec::from(data.as_ref()), path, CompressMethod::Deflated);
    }

    pub fn add_assets_from_reader<T: Read>(&mut self, name: &str, mut data: T) -> Result<(),std::io::Error> {
        let mut content: Vec<u8> = Vec::new();
        data.read_to_end(&mut content)?;
        let mut path = String::from("assets/");
        path.push_str(name);
        self.editor.append_file(content, path, CompressMethod::Deflated);
        Ok(())
    }

    pub fn add_file<T: AsRef<[u8]>>(&mut self, path: &str, data: T, compress_method: CompressMethod) {
        self.editor.append_file(Vec::from(data.as_ref()), String::from(path), compress_method);
    }

    pub fn edit_file<T: AsRef<[u8]>>(&mut self, path: &str, data: T) -> Option<()> {
        let raw = Vec::from(data.as_ref());
        self.editor.edit_file(&self.zip, path, raw)
    }

    pub fn remove_file(&mut self, path: &str) -> Option<()> {
        self.editor.remove_file(&self.zip, path)
    }

    pub fn save<W: Write>(&mut self, writer: W) -> Result<(), Box<dyn Error>> {
        self.editor.finish(Some(&self.zip), writer, 4)
    }

}
