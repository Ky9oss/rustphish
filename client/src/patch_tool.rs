use std::{fs, mem::size_of};
use std::error::Error;

async fn replace_url_in_exe(exe_path: &str, new_url: &str) -> Result<(), Box<dyn Error>> 
{
    // 读取二进制
    let mut data = fs::read(exe_path)?;

    // 搜索特征字节序列（如默认 URL）
    let default_url = b"https://default-url.com";
    let pattern = default_url.as_slice();
    let Some(offset) = find_subsequence(&data, pattern).await else {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "未找到目标缓冲区")));
    };

    // 验证新 URL 长度
    if new_url.len() + 1 > 256 { // +1 为终止符
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "URL 长度超过缓冲区容量")));
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
async fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_replace_url_in_exe() {
        // 创建临时文件
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        // 写入测试数据（包含默认URL）
        let mut test_data = vec![0u8; 1024];
        let default_url = b"https://default-url.com";
        test_data[256..256+default_url.len()].copy_from_slice(default_url);
        fs::write(path, &test_data).unwrap();

        // 测试替换URL
        let new_url = "https://new-test-url.com";
        replace_url_in_exe(path, new_url).await.unwrap();

        // 验证结果
        let updated_data = fs::read(path).unwrap();
        let expected_bytes = {
            let mut bytes = [0u8; 256];
            bytes[..new_url.len()].copy_from_slice(new_url.as_bytes());
            bytes[new_url.len()] = 0;
            bytes
        };

        assert_eq!(&updated_data[256..256+256], &expected_bytes[..]);
    }
}
