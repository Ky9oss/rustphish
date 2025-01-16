use lettre::message::header::ContentType;
use lettre::Message;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use std::error::Error;
use std::fs;

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
) -> Result<(), Box<dyn Error>> {
    // 验证凭证
    let creds = verify_smtp_credentials(smtp_server, username, password)?;
    
    // 读取HTML文件
    let html_content = fs::read_to_string(html_path)?;

    // 构建邮件
    let email = Message::builder()
        .from(from_email.parse()?)
        .to(to_email.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_content)?;

    // 创建SMTP传输器并发送邮件
    let mailer = SmtpTransport::relay(smtp_server)?
        .credentials(creds)
        .build();

    mailer.send(&email)?;

    Ok(())
}
