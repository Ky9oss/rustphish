use encoding_rs::GBK;


pub fn string_to_u8_32(s: &str) -> [u8; 32] {
    let mut array = [0u8; 32];  // 使用0来填充剩余字节
    let bytes = s.as_bytes();
    let len = bytes.len().min(32);  // 只取前32个字节
    array[..len].copy_from_slice(&bytes[..len]);
    array
}

pub fn u8_32_to_string(bytes: [u8; 32]) -> String {
    let first_zero_pos = bytes.iter().position(|&x| x == 0).unwrap_or(32);
    let valid_bytes = &bytes[..first_zero_pos];  // 只取到第一个零之前的字节
    String::from_utf8(valid_bytes.to_vec())
        .expect("Invalid UTF-8 sequence")  // 确保序列是有效的UTF-8
}

pub fn string_to_u8_16_gbk(s: &str) -> [u8; 16] {
    let mut array = [0u8; 16];
    let (encoded, _encoding_used, _had_errors) = GBK.encode(s);
    let bytes = encoded.as_ref();
    let len = bytes.len().min(16);
    array[..len].copy_from_slice(&bytes[..len]);
    array
}

pub fn u8_16_to_string_gbk(bytes: [u8; 16]) -> String {
    let first_zero_pos = bytes.iter().position(|&x| x == 0).unwrap_or(16);
    let valid_bytes = &bytes[..first_zero_pos];
    let (decoded, _encoding_used, _had_errors) = GBK.decode(valid_bytes);
    decoded.into()
}

pub fn string_to_u8_32_gbk(s: &str) -> [u8; 32] {
    let mut array = [0u8; 32];
    let (encoded, _encoding_used, _had_errors) = GBK.encode(s);
    let bytes = encoded.as_ref();
    let len = bytes.len().min(32);
    array[..len].copy_from_slice(&bytes[..len]);
    array
}


pub fn u8_32_to_string_gbk(bytes: [u8; 32]) -> String {
    let first_zero_pos = bytes.iter().position(|&x| x == 0).unwrap_or(32);
    let valid_bytes = &bytes[..first_zero_pos];
    let (decoded, _encoding_used, _had_errors) = GBK.decode(valid_bytes);
    decoded.into()
}

pub fn string_to_u8_64_gbk(s: &str) -> [u8; 64] {
    let mut array = [0u8; 64];
    let (encoded, _encoding_used, _had_errors) = GBK.encode(s);
    let bytes = encoded.as_ref();
    let len = bytes.len().min(64);
    array[..len].copy_from_slice(&bytes[..len]);
    array
}

pub fn string_to_u8_512_gbk(s: &str) -> [u8; 512] {
    let mut array = [0u8; 512];
    let (encoded, _encoding_used, _had_errors) = GBK.encode(s);
    let bytes = encoded.as_ref();
    let len = bytes.len().min(512);
    array[..len].copy_from_slice(&bytes[..len]);
    array
}

pub fn u8_512_to_string_gbk(bytes: [u8; 512]) -> String {
    let first_zero_pos = bytes.iter().position(|&x| x == 0).unwrap_or(512);
    let valid_bytes = &bytes[..first_zero_pos];
    let (decoded, _encoding_used, _had_errors) = GBK.decode(valid_bytes);
    decoded.into()
}

pub fn string_to_u8_4_gbk(s: &str) -> [u8; 4] {
    let mut array = [0u8; 4];
    let (encoded, _encoding_used, _had_errors) = GBK.encode(s);
    let bytes = encoded.as_ref();
    let len = bytes.len().min(4);
    array[..len].copy_from_slice(&bytes[..len]);
    array
}


pub fn u8_64_to_string_gbk(bytes: [u8; 64]) -> String {
    let first_zero_pos = bytes.iter().position(|&x| x == 0).unwrap_or(64);
    let valid_bytes = &bytes[..first_zero_pos];
    let (decoded, _encoding_used, _had_errors) = GBK.decode(valid_bytes);
    decoded.into()
}