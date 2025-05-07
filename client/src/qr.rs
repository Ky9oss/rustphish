use qrcode::QrCode;
use std::io::Cursor;
use image::{Luma, DynamicImage};
use base64::{Engine as _, engine::general_purpose};

#[cfg(feature = "mail")]
pub fn generate_qrcode_html(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 生成二维码
    let code = QrCode::new(url.as_bytes())?;
    
    // 渲染为图像
    let image_buffer = code.render::<Luma<u8>>()
        .quiet_zone(false)
        .min_dimensions(300, 300)
        .build();
    
    let dynamic_image = DynamicImage::ImageLuma8(image_buffer);

    // 写入字节流
    let mut bytes: Vec<u8> = Vec::new();
    // 创建内存缓冲区-带Seek Trait
    let mut writer = Cursor::new(&mut bytes);

    dynamic_image.write_to(&mut writer, image::ImageFormat::Png)?;
    
    // 生成base64编码字符串
    let base64_str = general_purpose::STANDARD.encode(&bytes);
    
    // 构建可直接嵌入HTML的data URL
    Ok(format!("data:image/png;base64,{}", base64_str))
}

