use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Write;
use byteorder::{LittleEndian, WriteBytesExt};
use flate2::write::DeflateDecoder;
use crate::utils::{get_leu32_value, get_leu16_value};
use crate::apk_zip::{CENTRAL_DIRECTORY, CENTRAL_DIRECTORY_END, CompressMethod, LOCAL_FILE_HEADER};

#[derive(Debug)]
pub struct ZipFormatError{
    offset: usize,
    reason: &'static str,
}

pub struct ZipEntry {
    pub(crate) origin_size: u32,
    pub(crate) compressed_size: u32,
    pub(crate) file_name: String,
    pub(crate) crc_32: u32,
    pub(crate) compress_method: CompressMethod,
    modify_time: u32,
    pub(crate) local_file_header_offset: u32,
    pub(crate) central_directory_header_offset: u32,
    pub(crate) entry_size: u32,
    pub(crate) ext_len: u16
}

pub struct ZipFile<'a> {
    pub(crate) data: &'a Vec<u8>,
    central_directory_offset: u32,
    pub(crate) entries: Vec<ZipEntry>,
    pub(crate) file_name_map: HashMap<String,usize>
}

pub(crate) struct LocalFileHeader {
    global_offset: usize,
    compress_version: u16,
    flags: u16,
    compress_method: CompressMethod,
    modify_time: u32,
    crc_32: u32,
    compressed_size: u32,
    origin_size: u32,
    file_name_len: u16,
    ext_len: u16,
    file_name: String,
    ext_data: Vec<u8>
}


impl Display for ZipFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "zip format error at: {}, reason: {}", self.offset, self.reason)
    }
}

impl Error for ZipFormatError {}

impl Clone for ZipEntry {
    fn clone(&self) -> Self {
        ZipEntry{
            origin_size: self.origin_size,
            compressed_size: self.compressed_size,
            file_name: self.file_name.clone(),
            crc_32: self.crc_32,
            compress_method: self.compress_method.clone(),
            modify_time: self.modify_time,
            local_file_header_offset: self.local_file_header_offset,
            central_directory_header_offset: self.central_directory_header_offset,
            entry_size: self.entry_size,
            ext_len: self.ext_len
        }
    }
}

impl LocalFileHeader {
    pub(crate) fn from_slice(data: &[u8], offset: usize) -> LocalFileHeader {
        // TODO unwrap
        let file_name_len = get_leu16_value(data, offset + 26);
        let ext_len = get_leu16_value(data, offset + 28);
        let file_name = String::from_utf8(data[(offset + 30)..(offset + 30 + file_name_len as usize)].to_vec()).unwrap();
        LocalFileHeader{
            global_offset: offset,
            compress_version: get_leu16_value(data, offset + 4),
            flags: get_leu16_value(data, offset + 6),
            compress_method: CompressMethod::convert_from_u16(get_leu16_value(data, offset + 8)).unwrap(),
            modify_time: get_leu32_value(data, offset + 10),
            crc_32: get_leu32_value(data, offset + 14),
            compressed_size: get_leu32_value(data, offset + 18),
            origin_size: get_leu32_value(data, offset + 22),
            file_name_len,
            ext_len,
            file_name,
            ext_data: data[(offset + 30 + file_name_len as usize)..(offset + 30 + (file_name_len + ext_len) as usize)].to_vec()
        }
    }

    pub(crate) fn write<W: Write>(&self, mut writer: W) -> Result<usize,std::io::Error> {
        writer.write_u32::<LittleEndian>(LOCAL_FILE_HEADER)?;
        writer.write_u16::<LittleEndian>(self.compress_version)?;
        writer.write_u16::<LittleEndian>(self.flags)?;
        writer.write_u16::<LittleEndian>(self.compress_method.value())?;
        writer.write_u32::<LittleEndian>(self.modify_time)?;
        writer.write_u32::<LittleEndian>(self.crc_32)?;
        writer.write_u32::<LittleEndian>(self.compressed_size)?;
        writer.write_u32::<LittleEndian>(self.origin_size)?;
        writer.write_u16::<LittleEndian>(self.file_name_len)?;
        writer.write_u16::<LittleEndian>(self.ext_len)?;
        writer.write_all(self.file_name.as_bytes())?;
        writer.write_all(self.ext_data.as_slice())?;
        Ok((self.file_name_len + self.ext_len + 30) as usize)
    }

    pub(crate) fn get_data_offset(&self) -> usize {
        self.global_offset + self.file_name_len as usize + self.ext_len as usize + 30
    }

    pub(crate) fn get_data_len(&self) -> u32 {
        self.compressed_size
    }

}

impl<'a> ZipFile<'a> {

    pub fn get_file_compress_data(&self, idx: usize) -> Option<&[u8]> {
        let header_offset = self.get_header_offset(idx)?;
        let file_name_len = get_leu16_value(self.data, (header_offset + 26) as usize) as u32;
        let ext_len = get_leu16_value(self.data, (header_offset + 28) as usize) as u32;
        let compress_size = get_leu32_value(self.data, (header_offset + 18) as usize);
        let file_start_offset = (header_offset + 30 + file_name_len + ext_len) as usize;
        Some(&self.data[file_start_offset..(file_start_offset + compress_size as usize)])
    }

    pub fn get_uncompress_data(&self, name: &str) -> Option<Vec<u8>> {
        let idx = *self.file_name_map.get(name)?;
        let compress_method = self.entries.get(idx)?.compress_method.clone();
        let raw = self.get_file_compress_data(idx)?;
        match compress_method {
            CompressMethod::Stored => Some(Vec::from(raw)),
            CompressMethod::Deflated => {
                let mut data: Vec<u8> = Vec::new();
                let mut decoder = DeflateDecoder::new(&mut data);
                decoder.write_all(raw);
                decoder.finish();
                Some(data)
            }
        }
    }

    pub fn get_entry_header_data(&self, idx: usize) -> Option<&[u8]> {
        let header_offset = self.get_header_offset(idx)?;
        let file_name_len = get_leu16_value(self.data, (header_offset + 26) as usize) as u32;
        let ext_len = get_leu16_value(self.data, (header_offset + 28) as usize) as u32;
        let end = (header_offset + 30 + file_name_len + ext_len) as usize;
        Some(&self.data[(header_offset as usize)..end])
    }

    pub fn get_header_offset(&self, idx: usize) -> Option<u32> {
        let entry  = self.entries.get(idx)?;
        Some(entry.local_file_header_offset)
    }

    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    pub fn get_entry(&self, idx: usize) -> Option<&ZipEntry> {
        self.entries.get(idx)
    }

    pub fn get_file(&self, name: &str) -> Option<&ZipEntry> {
        let idx = self.file_name_map.get(name)?;
        self.get_entry(*idx)
    }

    pub(crate) fn get_file_index(&self, name: &str) -> Option<usize> {
        Some(*(self.file_name_map.get(name)?))
    }

    pub fn from(data: &Vec<u8>) -> Result<ZipFile,ZipFormatError> {
        let mut res = ZipFile{
            data,
            central_directory_offset: 0,
            entries: vec![],
            file_name_map: HashMap::new()
        };

        let mut seek_index: usize = 0;
        let central_directory_end_offset = loop {
            let magic = get_leu32_value(data, data.len() - 22 - seek_index);
            if magic == CENTRAL_DIRECTORY_END {
                break data.len() - 22 - seek_index;
            }
            seek_index += 1;
            if (data.len() - 22 - seek_index < 4) || seek_index > 65535 {
                return Err(ZipFormatError{offset: data.len() - 22 - seek_index, reason: "Central directory end not found"})
            }
        };

        res.central_directory_offset = get_leu32_value(data, central_directory_end_offset + 16);
        let dir_count = get_leu16_value(data, central_directory_end_offset + 10);
        let mut current_offset = res.central_directory_offset as usize;
        let mut parse_count = 0;
        while parse_count < dir_count {

            if get_leu32_value(data, current_offset) != CENTRAL_DIRECTORY {
                return Err(ZipFormatError{
                    offset: current_offset,
                    reason: "magic of central directory error"
                });
            }

            let file_name_len = get_leu16_value(data, current_offset + 28);
            let ext_len = get_leu16_value(data, current_offset + 30);
            let comment_len = get_leu16_value(data, current_offset + 32);
            let file_name_data = data.as_slice()[(current_offset + 46)..(current_offset + 46 + file_name_len as usize)].to_vec();
            let file_name = match String::from_utf8(file_name_data){
                Ok(v) => v,
                Err(_) => return Err(ZipFormatError{
                    offset: current_offset,
                    reason: "convert string fail"
                })
            };
            res.file_name_map.insert(file_name.clone(), res.entries.len());

            let entry = ZipEntry{
                origin_size: get_leu32_value(data, current_offset + 24),
                compressed_size: get_leu32_value(data, current_offset + 20),
                file_name,
                crc_32: get_leu32_value(data, current_offset + 16),
                compress_method: CompressMethod::convert_from_u16(get_leu16_value(data, current_offset + 10)).unwrap(),
                modify_time: get_leu32_value(data, current_offset + 12),
                local_file_header_offset: get_leu32_value(data, current_offset + 42),
                central_directory_header_offset: current_offset as u32,
                entry_size: 46 + file_name_len as u32 + ext_len as u32 + comment_len as u32,
                ext_len
            };

            current_offset += entry.entry_size as usize;
            parse_count += 1;
            res.entries.push(entry);
        }
        Ok(res)
    }

}
