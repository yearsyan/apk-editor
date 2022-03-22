pub(crate) fn get_le32_value(data: &Vec<u8>, offset: usize) -> i32 {
    (data[offset] as i32) | ((data[offset + 1] as i32) << 8)
        | ((data[offset + 2] as i32) << 16) | ((data[offset + 3] as i32) << 24)
}

pub(crate) fn get_leu32_value<I: AsRef<[u8]>>(data: I, offset: usize) -> u32 {
    let data = data.as_ref();
    (data[offset] as u32) | ((data[offset + 1] as u32) << 8)
        | ((data[offset + 2] as u32) << 16) | ((data[offset + 3] as u32) << 24)
}


pub(crate) fn get_leu16_value<I: AsRef<[u8]>>(data: I, offset: usize) -> u16 {
    let data = data.as_ref();
    (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
}

pub(crate) fn push_le32 (data: &mut Vec<u8>, value: i32) {
    data.push((value & 0xff) as u8);
    data.push(((value >> 8) & 0xff) as u8);
    data.push(((value >> 16) & 0xff) as u8);
    data.push(((value >> 24) & 0xff) as u8);
}

pub fn push_leu32(data: &mut Vec<u8>, value: u32) {
    data.push((value & 0xff) as u8);
    data.push(((value >> 8) & 0xff) as u8);
    data.push(((value >> 16) & 0xff) as u8);
    data.push(((value >> 24) & 0xff) as u8);
}
