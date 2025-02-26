use std::{fs, mem::size_of};
use std::error::Error;

fn replace_url_in_exe(exe_path: &str, new_url: &str) -> Result<(), Box<dyn Error>> {
    // 读取二进制
    let mut data = fs::read(exe_path)?;

    // 搜索特征字节序列（如默认 URL）
    let default_url = b"https://default-url.com";
    let pattern = default_url.as_slice();
    let Some(offset) = find_subsequence(&data, pattern) else {
        return Err(Box::new("未找到目标缓冲区".into()));
    };

    // 验证新 URL 长度
    if new_url.len() + 1 > 256 { // +1 为终止符
        return Err(Box::new("URL 长度超过缓冲区容量".into()));
    }

    // 构造新字节（补零填充）
    let mut new_bytes = [0u8; 256];
    new_bytes[..new_url.len()].copy_from_slice(new_url.as_bytes());
    new_bytes[new_url.len()] = 0; // 终止符

    // 替换
    data[offset..offset + 256].copy_from_slice(&new_bytes);
    fs::write(exe_path, data)?;

    Ok(())
}

// 使用 Two-way 算法高效搜索字节序列
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// 运行示例
// fn main() -> Result<()> {
//     replace_url_in_exe("target/release/app.exe", "https://new-url.com")?;
//     Ok(())
// }
