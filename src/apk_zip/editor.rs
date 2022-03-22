use std::error::Error;
use std::io::Write;
use byteorder::{LittleEndian, WriteBytesExt};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use crate::apk_zip::{CENTRAL_DIRECTORY, CENTRAL_DIRECTORY_END, CompressMethod, LOCAL_FILE_HEADER};
use crate::apk_zip::zip::{LocalFileHeader, ZipEntry, ZipFile};
use crate::utils::{get_leu16_value};

struct AppendZipEntry {
    data: Vec<u8>,
    compress_method: CompressMethod,
    file_name: String,
    modify_time: u32
}

struct EditZipEntry {
    origin_entry: ZipEntry,
    remove: bool,
    edit: Option<Vec<u8>>
}

pub struct ZipEditor {
    // origin_zip: Option<&'a ZipFile<'a>>,
    editable_entries: Vec<EditZipEntry>,
    append_entries: Vec<AppendZipEntry>
}

struct FileHeaderBuilder<'a> {
    file_name: &'a str,
    compress_method: CompressMethod,
    origin_size: u32,
    compress_size: u32,
    crc32: u32,
    lfd_ext: Option<&'a [u8]>
}

impl<'a> FileHeaderBuilder<'a> {

    fn from_entry(zip: &'a ZipFile, entry: &'a ZipEntry) -> FileHeaderBuilder<'a> {
        let lfh_offset = entry.local_file_header_offset;
        let file_name_len = get_leu16_value(zip.data, lfh_offset as usize + 26);
        let ext_start = lfh_offset as usize + 30 + file_name_len as usize;
        let ext_len = get_leu16_value(zip.data, lfh_offset as usize + 28);
        let ext_end = ext_start + ext_len as usize;
        FileHeaderBuilder {
            file_name: entry.file_name.as_str(),
            compress_method: entry.compress_method.clone(),
            origin_size: entry.origin_size,
            compress_size: entry.compressed_size,
            crc32: entry.crc_32,
            lfd_ext: if ext_len == 0 {
                None
            } else {
                Some(&zip.data[ext_start..ext_end])
            }
        }
    }

    fn new(file_name: &'a str, compress_method: CompressMethod, origin_size: u32, compress_size: u32, crc32: u32) -> FileHeaderBuilder<'a> {
        FileHeaderBuilder{
            file_name,
            compress_method,
            origin_size,
            compress_size,
            crc32,
            lfd_ext: None
        }
    }

    fn set_compressed_size(&mut self, size: u32) {
        self.compress_size = size;
    }

    pub fn set_ldf_ext(&mut self, value: &'a [u8]) {
        self.lfd_ext = Some(value);
    }

    pub fn write_cd<W: Write>(&self, mut writer: W, lfh_offset: u32) -> Result<usize, std::io::Error> {
        writer.write_u32::<LittleEndian>(CENTRAL_DIRECTORY)?;
        writer.write_u16::<LittleEndian>(0)?;
        writer.write_u16::<LittleEndian>(0)?;
        writer.write_u16::<LittleEndian>(0)?; // flag
        writer.write_u16::<LittleEndian>(self.compress_method.value())?; // method
        writer.write_u32::<LittleEndian>(0)?; // modify
        writer.write_u32::<LittleEndian>(self.crc32)?;
        writer.write_u32::<LittleEndian>(self.compress_size)?;
        writer.write_u32::<LittleEndian>(self.origin_size)?;
        writer.write_u16::<LittleEndian>(self.file_name.len() as u16)?;
        writer.write_u16::<LittleEndian>(0)?; // ext len
        writer.write_u16::<LittleEndian>(0)?; // comment
        writer.write_u16::<LittleEndian>(0)?;
        writer.write_u16::<LittleEndian>(0)?; // internal
        writer.write_u32::<LittleEndian>(0)?; // external
        writer.write_u32::<LittleEndian>(lfh_offset)?;
        writer.write_all(self.file_name.as_bytes())?;
        Ok(46 + self.file_name.len())
    }

    pub fn write_lfh<W: Write>(&self, mut writer: W, offset: usize, align: usize) -> Result<usize, std::io::Error> {
        let origin_ext_len = match self.lfd_ext {
            Some(v) => v.len(),
            None => 0
        };
        let origin_lfd_len = 30 + self.file_name.len() + origin_ext_len;
        let align_count: usize = if self.compress_method != CompressMethod::Stored {
            0
        } else {
            (align - ((offset + origin_lfd_len) % align)) % align
        };
        let new_ext_len = origin_ext_len + align_count;
        writer.write_u32::<LittleEndian>(LOCAL_FILE_HEADER)?;
        writer.write_u16::<LittleEndian>(0)?;
        writer.write_u16::<LittleEndian>(0)?;
        writer.write_u16::<LittleEndian>(self.compress_method.value())?;
        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(self.crc32)?;
        writer.write_u32::<LittleEndian>(self.compress_size)?;
        writer.write_u32::<LittleEndian>(self.origin_size)?;
        writer.write_u16::<LittleEndian>(self.file_name.len() as u16)?;
        writer.write_u16::<LittleEndian>(new_ext_len as u16)?;
        writer.write_all(self.file_name.as_bytes())?;
        match self.lfd_ext {
            Some(ext_data) => writer.write_all(ext_data)?,
            _ => {}
        };
        for _ in 0.. align_count {
            writer.write_u8(0)?;
        }
        Ok(30 + self.file_name.len() + new_ext_len)
    }
}


impl ZipEditor {

    pub fn new() -> ZipEditor {
        ZipEditor{
            // origin_zip: None,
            editable_entries: vec![],
            append_entries: vec![]
        }
    }

    pub fn from(zip_file: & ZipFile) -> ZipEditor {
        let mut res = ZipEditor{
            // origin_zip: Some(zip_file),
            editable_entries: vec![],
            append_entries: vec![]
        };
        for entry in &zip_file.entries {
            res.editable_entries.push(EditZipEntry{
                origin_entry: entry.clone(),
                remove: false,
                edit: None
            });
        }
        res
    }

    pub fn append_file(&mut self, data: Vec<u8>, file_name: String, method: CompressMethod) {
        self.append_entries.push(AppendZipEntry{
            data,
            compress_method: method,
            file_name,
            modify_time: 0
        });
    }

    pub fn edit_file(&mut self, origin_zip: &ZipFile, name: &str, data: Vec<u8>) -> Option<()> {
        let idx = origin_zip.get_file_index(name)?;
        let mut item = self.editable_entries.get_mut(idx)?;
        item.edit = Some(data);
        Some(())
    }

    pub fn remove_file(&mut self, origin_zip: &ZipFile, name: &str) -> Option<()> {
        let idx = origin_zip.get_file_index(name)?;
        let mut item = self.editable_entries.get_mut(idx)?;
        item.remove = true;
        Some(())
    }

    pub fn finish<W: Write>(&self, origin_zip: Option<&ZipFile>, mut writer: W, align: usize) -> Result<(), Box<dyn Error>> {
        let mut central_directory_data: Vec<u8> = Vec::new();
        let mut current_offset: usize = 0;
        let mut file_count: u16 = 0;

        if origin_zip.is_some() {
            let origin_zip = origin_zip.unwrap();
            for entry in &self.editable_entries {
                if entry.remove {
                    continue;
                }

                file_count += 1;
                let lfh = LocalFileHeader::from_slice(origin_zip.data.as_slice(), entry.origin_entry.local_file_header_offset as usize);
                let mut header_build = FileHeaderBuilder::from_entry(origin_zip, &entry.origin_entry);
                let new_local_file_header_offset = current_offset as u32;
                if entry.edit.is_none() {
                    current_offset += header_build.write_lfh(&mut writer, current_offset, align)?;
                    let data_start = lfh.get_data_offset();
                    let data = &origin_zip.data[data_start..(data_start + lfh.get_data_len() as usize)];
                    writer.write_all(data)?;
                    current_offset += data.len();
                } else {
                    let new_file = entry.edit.as_ref().unwrap();
                    if entry.origin_entry.compress_method == CompressMethod::Stored {
                        header_build.set_compressed_size(new_file.len() as u32);
                        current_offset += header_build.write_lfh(&mut writer, current_offset, align)?;
                        writer.write_all(new_file.as_slice())?;
                        current_offset += new_file.len();
                    } else {
                        let mut hasher = crc32fast::Hasher::new();
                        hasher.update(entry.edit.as_ref().unwrap().as_slice());
                        let crc32 = hasher.finalize();

                        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
                        encoder.write_all(entry.edit.as_ref().unwrap().as_slice())?;
                        let compress_data = encoder.finish()?;

                        header_build.origin_size = entry.edit.as_ref().unwrap().len() as u32;
                        header_build.set_compressed_size(compress_data.len() as u32);
                        header_build.crc32 = crc32;

                        current_offset += header_build.write_lfh(&mut writer, current_offset, align)?;
                        writer.write_all(compress_data.as_slice())?;
                        current_offset += compress_data.as_slice().len();
                    }

                }
                header_build.write_cd(&mut central_directory_data, new_local_file_header_offset)?;
            }
        }

        for new_entry in &self.append_entries {
            file_count += 1;

            let mut hash = crc32fast::Hasher::new();
            hash.update(new_entry.data.as_slice());
            let crc32_hash = hash.finalize();

            let mut compress_data_opt: Option<Vec<u8>> = None;
            if new_entry.compress_method != CompressMethod::Stored {
                let mut compress_data: Vec<u8> = Vec::new();
                let mut encoder = DeflateEncoder::new(&mut compress_data, Compression::default());
                encoder.write_all(new_entry.data.as_slice())?;
                encoder.finish()?;
                compress_data_opt = Some(compress_data);
            }

            let file_header = FileHeaderBuilder::new(
                new_entry.file_name.as_str(),
                new_entry.compress_method.clone(),
                new_entry.data.len() as u32,
                match &compress_data_opt {
                    Some(data) => data.len(),
                    None => new_entry.data.len()
                } as u32,
                crc32_hash
            );

            file_header.write_cd(&mut central_directory_data, current_offset as u32)?;
            current_offset += file_header.write_lfh(&mut writer, current_offset, align)?;

            if new_entry.compress_method == CompressMethod::Stored {
                writer.write_all(new_entry.data.as_slice())?;
                current_offset += new_entry.data.len();
            } else {
                writer.write_all(compress_data_opt.as_ref().unwrap().as_slice())?;
                current_offset += compress_data_opt.unwrap().len();
            }
        }

        let central_directory_offset = current_offset as u32;
        writer.write_all(central_directory_data.as_slice())?;
        writer.write_u32::<LittleEndian>(CENTRAL_DIRECTORY_END)?;
        writer.write_u16::<LittleEndian>(0)?;
        writer.write_u16::<LittleEndian>(0)?;
        writer.write_u16::<LittleEndian>(file_count)?;
        writer.write_u16::<LittleEndian>(file_count)?;
        writer.write_u32::<LittleEndian>(central_directory_data.len() as u32)?;
        writer.write_u32::<LittleEndian>(central_directory_offset)?;
        writer.write_u16::<LittleEndian>(0)?;
        Ok(())
    }
}
