use std::string::FromUtf16Error;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Write;
use byteorder::{LittleEndian, WriteBytesExt};
use crate::utils::{*};

const START_TAG: i32 = 0x00100102;
const END_TAG: i32 = 0x00100103;
const START_NAMESPACE: i32 = 0x00100100;
const END_NAMESPACE: i32 = 0x00100101;
const STRING_CHUNK: i32 = 0x001C0001;
const RESOURCE_CHUNK: i32 = 0x00080180;
const XML_MAGIC: i32 = 0x00080003;

#[derive(Debug)]
pub struct FileFormatError{
    offset: usize
}


pub struct XmlAttributeValue {
    pub(crate) namespace_uri: Option<String>, // AndroidManifest http://schemas.android.com/apk/res/android
    pub(crate) name_index: u32,
    pub(crate) name: String,
    pub(crate) value_type: u32,
    pub(crate) string_data: Option<String>,
    pub(crate) data: u32
}

pub struct XmlNode {
    pub(crate) tag_name: String,
    pub(crate) attrs: Vec<XmlAttributeValue>,
    pub(crate) children: Vec<Box<XmlNode>>
}


pub struct StringChunk<'a> {
    data: &'a Vec<u8>,
    chunk_offset: usize,
    chunk_size: u32,
    string_count: u32,
    style_count: u32,
    string_pool_offset: u32,
    style_pool_offset: u32,
    string_index_global_offset: usize,
    style_index_global_offset: usize
}

pub struct ResourceChunk<'a> {
    data: &'a Vec<u8>,
    chunk_offset: usize,
    chunk_size: u32,
    chunk_count: u32
}

pub struct XmlContent {
    namespace_prefix: String,
    namespace_uri: String,
    pub(crate) root_node: Box<XmlNode>,
}

pub struct XmlNameSpace<'a> {
    data: &'a Vec<u8>,
    namespace_offset: usize,
    line_number: u32,
    prefix: String,
    uri: String
}

pub struct AndroidXml<'a> {
    data: &'a Vec<u8>,
    pub(crate) string_chunk: Box<StringChunk<'a>>,
    resource_chunk: Box<ResourceChunk<'a>>,
    pub(crate) content: Box<XmlContent>
}

pub struct StringChunkBuilder {
    string_index_map: HashMap<String,u32>,
    string_arr: Vec<String>
}

impl Display for FileFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "file format error at: {}", self.offset)
    }
}

impl Error for FileFormatError {}

impl StringChunkBuilder {
    pub fn build(&self) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();
        push_le32(&mut res, STRING_CHUNK);
        push_le32(&mut res, 0); // size
        push_leu32(&mut res, self.string_arr.len() as u32);
        push_leu32(&mut res, 0);
        push_leu32(&mut res, 0);
        push_leu32(&mut res, (7 * 4 + self.string_arr.len() * 4) as u32); // string pool offset
        push_leu32(&mut res, 0); // style pool offset
        let mut current_str_offset: u32 = 0;
        for str_item in &self.string_arr {
            push_leu32(&mut res, current_str_offset);
            current_str_offset += (2 + str_item.len()*2 + 2) as u32;
        }
        for str_item in &self.string_arr {
            let str_len = str_item.len();
            res.push((str_len & 0xff) as u8);
            res.push(((str_len >> 8) & 0xff) as u8);
            let str_data: Vec<u16> = str_item.encode_utf16().collect();
            for ch in str_data {
                res.push((ch & 0xff) as u8);
                res.push(((ch >> 8) & 0xff) as u8);
            }
            res.push(0);
            res.push(0);
        }
        let align_len = 4 - (res.len() % 4);
        if align_len < 4 {
            for i in 0..align_len {
                res.push(0);
            }
        }
        let chunk_len = res.len();
        res[4] = (chunk_len & 0xff) as u8;
        res[5] = ((chunk_len >> 8) & 0xff) as u8;
        res[6] = ((chunk_len >> 16) & 0xff) as u8;
        res[7] = ((chunk_len >> 24) & 0xff) as u8;
        res
    }
    pub(crate) fn put(&mut self, value: &str) -> u32 {
        if self.string_index_map.contains_key(value) {
            return self.string_index_map.get(value).unwrap().clone();
        }
        let res = self.string_index_map.len() as u32;
        self.string_index_map.insert(String::from(value), res);
        self.string_arr.push(String::from(value));
        return res;
    }

    pub fn new() -> StringChunkBuilder {
        StringChunkBuilder{
            string_index_map: HashMap::new(),
            string_arr: Vec::new()
        }
    }

    pub(crate) fn init(&mut self, string_chunk: &StringChunk) {
        for i in 0..string_chunk.string_count {
            self.put(string_chunk.get_string(i).unwrap().as_str());
        }
    }

    pub fn from_string_chunk(string_chunk: &StringChunk) -> StringChunkBuilder {
        let mut res = StringChunkBuilder{
            string_index_map: HashMap::new(),
            string_arr: Vec::new()
        };
        for i in 0..string_chunk.string_count {
            res.put(string_chunk.get_string(i).unwrap().as_str());
        }
        res
    }
}

impl XmlAttributeValue {
    pub fn new_attr(idx: u32, name: &str, value: &str, string_chunk_builder: &mut StringChunkBuilder) -> XmlAttributeValue {
        XmlAttributeValue{
            namespace_uri: Some("http://schemas.android.com/apk/res/android".to_string()),
            name: String::from(name),
            name_index: idx,
            value_type: 0x3000008,
            string_data: Some(String::from(value)),
            data: string_chunk_builder.put(value)
        }
    }

    pub fn new_name_attr(value: &str, string_chunk_builder: &mut StringChunkBuilder) -> XmlAttributeValue {
        XmlAttributeValue::new_attr(3, "name", value, string_chunk_builder)
    }

    pub fn new_authorities_attr(value: &str, string_chunk_builder: &mut StringChunkBuilder) -> XmlAttributeValue {
        XmlAttributeValue::new_attr(5, "authorities", value, string_chunk_builder)
    }
}

impl XmlNode {

    pub fn walk_children<F>(&mut self, mut f: F) where F: FnMut(&mut Box<XmlNode>) {
        for child in &mut self.children {
            f(child);
        }
    }

    pub fn push_child(&mut self, new_child: Box<XmlNode>) {
        self.children.push(new_child);
    }

    fn parse_node_recursion(data: &Vec<u8>, string_chunk: &StringChunk, current_offset: & mut usize) -> Result<Box<XmlNode>, Box<dyn Error>> {
        let tag_type = get_le32_value(data, *current_offset);
        // let line_no = get_le32_value(data, *current_offset + 2 * 4);
        let name_si = get_leu32_value(data, *current_offset + 5 * 4);
        let mut res = XmlNode{
            tag_name: String::new(),
            attrs: vec![],
            children: vec![]
        };

        let tag_name : String;
        if tag_type == START_TAG {
            let attr_number = get_le32_value(data, *current_offset + 7 * 4);
            *current_offset += 9 * 4;
            tag_name = string_chunk.get_string(name_si)?;
            res.tag_name = tag_name.clone();

            for _ in 0..attr_number {
                let namespace_si = get_leu32_value(data, *current_offset);
                let attr_name_si = get_leu32_value(data, *current_offset + 1 * 4);
                let attr_raw_value = get_leu32_value(data, *current_offset + 2 * 4);
                let value_type =  get_leu32_value(data, *current_offset + 3 * 4);
                let attr_data = get_leu32_value(data, *current_offset + 4 * 4);
                let attr_name = string_chunk.get_string(attr_name_si)?;
                *current_offset += 5 * 4;

                res.attrs.push(XmlAttributeValue{
                    namespace_uri: if namespace_si == 0xffffffff {
                        None
                    } else {
                        Some(string_chunk.get_string(namespace_si)?)
                    },
                    name_index: attr_name_si,
                    name: attr_name,
                    value_type,
                    string_data: if attr_raw_value == 0xffffffff {
                        None
                    } else {
                        Some(string_chunk.get_string(attr_raw_value)?)
                    },
                    data: attr_data
                });
            }
        } else {
            return Err(Box::new(FileFormatError{ offset: *current_offset }))
        }

        while *current_offset < data.len() {
            let current_tag_type = get_le32_value(data, *current_offset);
            if current_tag_type == START_TAG {
                res.children.push(XmlNode::parse_node_recursion(data, string_chunk, current_offset)?);
            } else if current_tag_type == END_TAG {
                let current_name_si = get_leu32_value(data, *current_offset + 5 * 4);
                let current_name = string_chunk.get_string(current_name_si)?;
                *current_offset += 6 * 4;
                if current_name == tag_name {
                    return Ok(Box::new(res));
                }
            } else {
                return Err(Box::new(FileFormatError{ offset: *current_offset }));
            }
        }

        Ok(Box::new(res))

    }

    fn write<W: Write>(&self, mut writer: W, string_chunk_builder: &mut StringChunkBuilder) -> Result<(),std::io::Error> {
        writer.write_u32::<LittleEndian>(START_TAG as u32)?;
        writer.write_u32::<LittleEndian>(9 * 4 + (self.attrs.len() * 5 * 4) as u32)?;
        writer.write_u32::<LittleEndian>(1)?;
        writer.write_u32::<LittleEndian>(0xFFFFFFFF)?;
        writer.write_u32::<LittleEndian>(0xFFFFFFFF)?; //namesapce
        writer.write_u32::<LittleEndian>(string_chunk_builder.put(self.tag_name.as_str()))?;
        writer.write_u32::<LittleEndian>(0x00140014)?; // flag
        writer.write_u32::<LittleEndian>(self.attrs.len() as u32)?;
        writer.write_u32::<LittleEndian>(0)?;

        for attr in &self.attrs {
            writer.write_u32::<LittleEndian>(match &attr.namespace_uri {
                Some(namespace_str) => string_chunk_builder.put(namespace_str.as_str()),
                None => 0xFFFFFFFF
            })?;
            writer.write_u32::<LittleEndian>(attr.name_index)?;
            writer.write_u32::<LittleEndian>(match &attr.string_data {
                Some(value_str) => string_chunk_builder.put(value_str.as_str()),
                None => 0xFFFFFFFF
            })?;
            writer.write_u32::<LittleEndian>(attr.value_type)?;
            writer.write_u32::<LittleEndian>(attr.data)?;
        }

        for child in &self.children {
            child.write(&mut writer, string_chunk_builder)?;
        }

        writer.write_u32::<LittleEndian>(END_TAG as u32)?;
        writer.write_u32::<LittleEndian>(6 * 4)?;
        writer.write_u32::<LittleEndian>(1)?;
        writer.write_u32::<LittleEndian>(0xFFFFFFFF)?;
        writer.write_u32::<LittleEndian>(0xFFFFFFFF)?; // namespace
        writer.write_u32::<LittleEndian>(string_chunk_builder.put(self.tag_name.as_str()))?;

        Ok(())
    }

    fn regenerate(&self, data: &mut Vec<u8>, string_chunk_builder: &mut StringChunkBuilder) {
        push_le32(data, START_TAG);
        push_leu32(data, 9 * 4 + (self.attrs.len() * 5 * 4) as u32);
        push_leu32(data, 1);
        push_leu32(data, 0xFFFFFFFF);
        push_leu32(data, 0xFFFFFFFF); // namespace
        push_leu32(data, string_chunk_builder.put(self.tag_name.as_str()));
        push_leu32(data, 0x00140014); // flag
        push_leu32(data, self.attrs.len() as u32);
        push_leu32(data, 0);

        for attr in &self.attrs {
            push_leu32(data, match &attr.namespace_uri {
                Some(namespace_str) => string_chunk_builder.put(namespace_str.as_str()),
                None => 0xFFFFFFFF
            });
            push_leu32(data, attr.name_index);
            match &attr.string_data {
                Some(value_str) => push_leu32(data, string_chunk_builder.put(value_str.as_str())),
                None => push_leu32(data, 0xFFFFFFFF)
            }
            push_leu32(data, attr.value_type);
            push_leu32(data, attr.data);
        }

        for child in &self.children {
            child.regenerate(data, string_chunk_builder);
        }

        push_le32(data, END_TAG);
        push_leu32(data, 6 * 4);
        push_leu32(data, 1);
        push_leu32(data, 0xFFFFFFFF);
        push_leu32(data, 0xFFFFFFFF); // namespace
        push_leu32(data, string_chunk_builder.put(self.tag_name.as_str()));

    }

}

impl XmlContent {
    fn parse<'a>(data: &'a Vec<u8>, string_chunk: &StringChunk, current_offset: &mut usize) -> Result<Box<XmlContent>, Box<dyn Error>> {
        let namespace = XmlNameSpace::parse(data, string_chunk, current_offset)?;
        let root = XmlNode::parse_node_recursion(data, string_chunk, current_offset)?;
        namespace.valid_end_chunk(data, string_chunk, current_offset)?;
        Ok(Box::new(XmlContent{
            namespace_prefix: namespace.prefix,
            namespace_uri: namespace.uri,
            root_node: root
        }))
    }

    fn to_data(&self, string_chunk_builder: &mut StringChunkBuilder) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();

        // start namespace
        push_le32(&mut res, START_NAMESPACE);
        push_leu32(&mut res, 4 * 6);
        push_leu32(&mut res, 1); // line number
        push_leu32(&mut res, 0xFFFFFFFF);
        push_leu32(&mut res, string_chunk_builder.put(self.namespace_prefix.as_str()));
        push_leu32(&mut res, string_chunk_builder.put(self.namespace_uri.as_str()));

        self.root_node.regenerate(&mut res, string_chunk_builder);

        // end namespace
        push_le32(&mut res, END_NAMESPACE);
        push_leu32(&mut res, 4 * 6);
        push_leu32(&mut res, 1); // line number
        push_leu32(&mut res, 0xFFFFFFFF);
        push_leu32(&mut res, string_chunk_builder.put(self.namespace_prefix.as_str()));
        push_leu32(&mut res, string_chunk_builder.put(self.namespace_uri.as_str()));
        res
    }
}

impl XmlNameSpace<'_> {
    fn parse<'a>(data: &'a Vec<u8>,string_chunk: &StringChunk, current_offset: &mut usize) -> Result<Box<XmlNameSpace<'a>>, Box<dyn Error>> {
        if get_le32_value(data, *current_offset) != START_NAMESPACE {
            return Err(Box::new(FileFormatError{offset: *current_offset}));
        }
        let res = XmlNameSpace{
            data,
            namespace_offset: *current_offset,
            line_number: get_leu32_value(data, *current_offset + 2 * 4),
            prefix: string_chunk.get_string(get_leu32_value(data, *current_offset + 4 * 4))?,
            uri: string_chunk.get_string(get_leu32_value(data, *current_offset + 5 * 4))?
        };
        *current_offset += get_leu32_value(data, *current_offset + 4) as usize;
        Ok(Box::new(res))
    }

    fn valid_end_chunk<'a>(&self, data: &'a Vec<u8>,string_chunk: &StringChunk, current_offset: &mut usize) -> Result<(), Box<dyn Error>> {
        if get_le32_value(data, *current_offset) != END_NAMESPACE {
            return Err(Box::new(FileFormatError{offset: *current_offset}));
        }
        let prefix = string_chunk.get_string(get_leu32_value(data, *current_offset + 4 * 4))?;
        let uri = string_chunk.get_string(get_leu32_value(data, *current_offset + 5 * 4))?;
        if prefix != self.prefix || uri != self.uri {
            return Err(Box::new(FileFormatError{offset: *current_offset}));
        }
        Ok(())
    }
}

impl ResourceChunk<'_> {
    fn parse<'a>(data: &'a Vec<u8>, current_offset: &mut usize) -> Result<Box<ResourceChunk<'a>>,Box<dyn Error>> {
        let mut res = ResourceChunk{
            data,
            chunk_offset: *current_offset,
            chunk_size: get_leu32_value(data, *current_offset + 4),
            chunk_count: 0
        };
        if (get_le32_value(data, *current_offset)) != RESOURCE_CHUNK {
            return Err(Box::new(FileFormatError{offset: *current_offset}))
        }
        res.chunk_count = res.chunk_size/4 - 2;
        *current_offset = *current_offset + res.chunk_size as usize;
        Ok(Box::new(res))
    }
}

impl StringChunk<'_> {
    fn parse<'a>(data: &'a Vec<u8>, current_offset: &mut usize) -> Result<Box<StringChunk<'a>>,Box<dyn Error>> {
        let mut res = StringChunk{
            data,
            chunk_offset: *current_offset,
            chunk_size: 0,
            string_count: 0,
            style_count: 0,
            string_pool_offset: 0,
            style_pool_offset: 0,
            string_index_global_offset: 0,
            style_index_global_offset: 0
        };
        let chunk_type = get_le32_value(data, *current_offset);
        if chunk_type != STRING_CHUNK {
            return Err(Box::new(FileFormatError{offset: *current_offset}));
        }
        *current_offset += 4;
        res.chunk_size = get_leu32_value(data, *current_offset);
        *current_offset += 4;
        res.string_count = get_leu32_value(data, *current_offset);
        *current_offset += 4;
        res.style_count = get_leu32_value(data, *current_offset);
        *current_offset += 8; // 4 byte unknown
        res.string_pool_offset = get_leu32_value(data, *current_offset);
        *current_offset += 4;
        res.style_pool_offset = get_leu32_value(data, *current_offset);
        *current_offset += 4;
        res.string_index_global_offset = *current_offset;
        *current_offset += 4;
        res.style_index_global_offset = *current_offset;
        *current_offset = res.chunk_offset + (res.chunk_size as usize);
        Ok(Box::new(res))
    }

    fn get_string(&self, index: u32) -> Result<String, FromUtf16Error> {
        let string_offset = (self.string_pool_offset as usize) + self.chunk_offset + get_leu32_value(self.data, self.string_index_global_offset + (4 * index as usize)) as usize;
        let string_len = (self.data[string_offset as usize] as u16) | ((self.data[(string_offset + 1) as usize] as u16) << 8);
        let mut utf_16_data : Vec<u16> = Vec::new();
        for i in 0..string_len {
            let char_index = (string_offset + 2 + ((i * 2) as usize)) as usize;
            let c = (self.data[char_index] as u16) | ((self.data[char_index + 1] as u16) << 8);
            utf_16_data.push(c);
        }
        String::from_utf16(utf_16_data.as_slice())
    }

}

impl XmlNode {
    fn push_data(&self, res: &mut String) {
        res.push('<');
        res.push_str(self.tag_name.as_str());
        res.push(' ');
        for k in &self.attrs {
            res.push_str(k.name.as_str());
            res.push_str("=\"");
            match &k.string_data{
                Some(s) => res.push_str(s.as_str()),
                None => res.push_str( k.data.to_string().as_str())
            }
            res.push('"');
            res.push(' ');
        }
        res.push('>');

        for child in &self.children {
            child.push_data(res);
        }
        res.push_str("</");
        res.push_str(self.tag_name.as_str());
        res.push_str(">");
    }
}


impl AndroidXml<'_> {
    pub fn from_data(data: &Vec<u8>) -> Result<AndroidXml, Box<dyn Error>> {
        let mut current_offset : usize = 0;
        let magic = get_le32_value(data, current_offset);
        if magic != XML_MAGIC {
            return Err(Box::new(FileFormatError{offset: 0}))
        }
        current_offset += 4;
        let file_length = get_le32_value(data, current_offset);
        if file_length as usize != data.len() {
            return Err(Box::new(FileFormatError{offset: current_offset}))
        }
        current_offset += 4;
        let string_chunk = StringChunk::parse(data, &mut current_offset)?;
        let resource_chunk = ResourceChunk::parse(data, &mut current_offset)?;
        let content = XmlContent::parse(data, &string_chunk, &mut current_offset)?;

        Ok(AndroidXml{
            data,
            string_chunk,
            resource_chunk,
            content
        })
    }

    pub fn regenerate(&self,string_chunk_builder: &mut StringChunkBuilder) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();
        push_le32(&mut res, XML_MAGIC);

        let content_data = self.content.to_data(string_chunk_builder);
        let string_chunk_data = string_chunk_builder.build();
        let file_size = 4 * 2 + string_chunk_data.len() + self.resource_chunk.chunk_size as usize +
            content_data.len();

        push_leu32(&mut res, file_size as u32);
        res.extend(string_chunk_data);
        for i in 0..self.resource_chunk.chunk_size {
            res.push(self.data[self.resource_chunk.chunk_offset + i as usize]);
        }
        res.extend(content_data);
        res
    }
}

impl Display for AndroidXml<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        self.content.root_node.push_data(&mut s);
        write!(f, "{}", s)
    }
}
