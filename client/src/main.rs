use std::error::Error;
use clap::{Command, Arg};
use colored::*;
use std::fs::{self, create_dir_all};
use std::path::Path;
use std::thread;
use std::time::Duration;
use serde::Deserialize;

#[cfg(feature = "mail")]
use rpassword::read_password;

#[cfg(feature = "mail")]
mod smtp;

#[cfg(feature = "mail")]
mod patch_tool;

#[cfg(feature = "db")]
mod db;

#[derive(Deserialize)]
struct Config {
    smtp: SmtpConfig,
    email: EmailConfig,
}

#[derive(Deserialize)]
struct SmtpConfig {
    server: String,
    username: String,
    from_email: String,
    subject: String,
    interval: u64,  // 发送间隔（秒）
}

#[derive(Deserialize)]
struct EmailConfig {
    template: String,  // 邮件模板路径
}

// mod db;
// mod smtp;

const BANNER: &str = r#"
██████╗ ██╗   ██╗███████╗████████╗██████╗ ██╗  ██╗██╗███████╗██╗  ██╗
██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔══██╗██║  ██║██║██╔════╝██║  ██║
██████╔╝██║   ██║███████╗   ██║   ██████╔╝███████║██║███████╗███████║
██╔══██╗██║   ██║╚════██║   ██║   ██╔═══╝ ██╔══██║██║╚════██║██╔══██║
██║  ██║╚██████╔╝███████║   ██║   ██║     ██║  ██║██║███████║██║  ██║
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝  ╚═╝
                                                            [Client v1.0]
"#;

fn print_status(status: &str, message: &str) {
    println!("[{}] {}", status.bold(), message);
}

fn print_success(message: &str) {
    print_status(&"✓".green(), message);
}

fn print_error(message: &str) {
    print_status(&"✗".red(), message);
}

fn print_info(message: &str) {
    print_status(&"i".blue(), message);
}

fn generate_phishing_emails(email_tree: &sled::Tree, template_path: &str) -> Result<(), Box<dyn Error>> {
    let output_dir = Path::new("./generate");
    create_dir_all(output_dir)?;

    let template = fs::read_to_string(template_path)?;
    let emails = db::get_all_emails(&email_tree)?;

    print_info(&format!("找到 {} 个目标邮箱", emails.len()));
    
    for entry in emails {
        let content = template.replace("{{id}}", &entry.id);
        let file_name = format!("generate/{}.html", entry.email);
        fs::write(&file_name, content)?;
        print_success(&format!("生成 {}", file_name));
    }

    Ok(())
}

#[cfg(feature = "mail")]
async fn send_multi_emails(email_tree: &sled::Tree, config: Config, password: String, from: u16, to: u16) -> Result<(), Box<dyn Error>> {
    let emails = db::get_all_emails(&email_tree)?;
    let end = std::cmp::min(to, emails.len().try_into().unwrap());
    let new_emails = &emails[from as usize..end as usize];

    print_info(&format!("找到 {} 个目标邮箱", new_emails.len()));

    // 读取邮件模板
    let template = fs::read_to_string(&config.email.template)?;

    // 验证SMTP凭证
    print_info("验证SMTP凭证...");
    smtp::verify_smtp_credentials(
        &config.smtp.server,
        &config.smtp.username,
        &password,
    )?;
    print_success("SMTP凭证验证成功");

    // 发送邮件
    for entry in new_emails {
        let content = template.replace("{{id}}", &entry.id);
        
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
        ) {
            Ok(result) => {
                print_success(&format!("发送成功: {}", entry.email))
            },
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
async fn send_phishing_emails(email_tree: &sled::Tree, config: Config, password: String) -> Result<(), Box<dyn Error>> {
    let emails = db::get_all_emails(&email_tree)?;

    print_info(&format!("找到 {} 个目标邮箱", emails.len()));

    // 读取邮件模板
    let template = fs::read_to_string(&config.email.template)?;

    // 验证SMTP凭证
    print_info("验证SMTP凭证...");
    smtp::verify_smtp_credentials(
        &config.smtp.server,
        &config.smtp.username,
        &password,
    )?;
    print_success("SMTP凭证验证成功");

    // 发送邮件
    for entry in emails {
        let content = template.replace("{{id}}", &entry.id);
        
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
        ) {
            Ok(result) => {
                print_success(&format!("发送成功: {}", entry.email))
            },
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

#[cfg(feature = "mail")]
async fn send_single_email(
    email_tree: &sled::Tree,
    config: &Config,
    target_id: &str,
    password: &str
) -> Result<(), Box<dyn Error>> {
    // 查找目标邮箱
    match email_tree.get(target_id.as_bytes())? {
        Some(value) => {
            let entry: db::EmailEntry = bincode::deserialize(&value)?;
            let template = fs::read_to_string(&config.email.template)?;
            let content = template.replace("{{id}}", &entry.id);
            
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
            ) {
                Ok(_) => print_success(&format!("发送成功: {}", entry.email)),
                Err(e) => print_error(&format!("发送失败 {}: {}", entry.email, e)),
            }

            // 清理临时文件
            fs::remove_file(&temp_file)?;
            Ok(())
        },
        None => {
            print_error(&format!("未找到ID为 {} 的邮箱", target_id));
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Email ID not found"
            )))
        }
    }
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
            .arg(Arg::new("read")
                .short('r')
                .long("read")
                .value_name("DATABASE")
                .help("读取并格式化显示钓鱼记录"))
            .arg(Arg::new("input")
                .short('i')
                .long("input")
                .value_name("EMAIL_LIST")
                .help("从文件导入目标邮箱列表"))
            .arg(Arg::new("show")
                .long("show")
                .help("显示所有目标邮箱")
                .num_args(0))
            .arg(Arg::new("delete")
                .short('d')
                .long("delete")
                .value_name("ID")
                .help("删除指定ID的邮箱记录"));
    }

    #[cfg(feature = "mail")]
    {
        app = app
            .arg(Arg::new("generate")
                .short('g')
                .long("generate")
                .value_name("TEMPLATE")
                .help("根据模板生成钓鱼邮件"))
            .arg(Arg::new("send-all")
                .long("send-all")
                .help("向所有目标发送钓鱼邮件")
                .num_args(0))
            .arg(Arg::new("send")
                .long("send")
                .value_name("ID")
                .help("向指定ID的目标发送钓鱼邮件"))
            .arg(Arg::new("send-from-to")
                .long("send-from-to")
                .value_name("ft")
                .help("发送邮件区间"))
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
                },
                Err(e) => {
                    print_error(&format!("打开服务器数据库失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let action_tree = match db_server.open_tree("actions") {
                Ok(tree) => {
                    tree
                },
                Err(e) => {
                    print_error(&format!("打开actions表失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let data_tree = match db_server.open_tree("data") {
                Ok(tree) => {
                    tree
                },
                Err(e) => {
                    print_error(&format!("打开data表失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let db_client = match sled::open(DB_PATH) {
                Ok(db) => {
                    db
                },
                Err(e) => {
                    print_error(&format!("打开客户端数据库失败: {}", e));
                    return Err(Box::new(e));
                }
            };

            let email_tree = match db_client.open_tree(EMAIL_TREE) {
                Ok(tree) => {
                    tree
                },
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
            print_info(&format!("正在生成钓鱼邮件: {}", template_path));
            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;
            
            match generate_phishing_emails(&email_tree, template_path) {
                Ok(_) => print_success("所有钓鱼邮件生成完成"),
                Err(e) => print_error(&format!("生成失败: {}", e)),
            }
        } else if matches.get_flag("send-all") {
            // 检查配置文件
            let config_path = "config.toml";
            if !Path::new(config_path).exists() {
                print_error("找不到配置文件 config.toml");
                return Ok(());
            }

            let config: Config = toml::from_str(&fs::read_to_string(config_path)?)?;

            if !Path::new(&config.email.template).exists() {
                print_error(&format!("找不到邮件模板文件 {}", config.email.template));
                return Ok(());
            }

            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            print_info("请输入SMTP密码：");
            let password = read_password()?;

            print_info("开始批量发送邮件");
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                send_phishing_emails(&email_tree, config, password).await?;
                Ok::<(), Box<dyn Error>>(())
            })?;
            print_success("所有邮件发送完成");
        } else if let Some(target_id) = matches.get_one::<String>("send") {
            // 检查配置文件
            let config_path = "config.toml";
            if !Path::new(config_path).exists() {
                print_error("找不到配置文件 config.toml");
                return Ok(());
            }

            let config: Config = toml::from_str(&fs::read_to_string(config_path)?)?;

            if !Path::new(&config.email.template).exists() {
                print_error(&format!("找不到邮件模板文件 {}", config.email.template));
                return Ok(());
            }

            let db = sled::open(DB_PATH)?;
            let email_tree = db.open_tree(EMAIL_TREE)?;

            print_info("请输入SMTP密码：");
            let password = read_password()?;

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                send_single_email(&email_tree, &config, target_id, &password).await?;
                Ok::<(), Box<dyn Error>>(())
            })?;
        } else if let Some(ft) = matches.get_one::<String>("send-from-to") {
            // 检查配置文件
            let config_path = "config.toml";
            if !Path::new(config_path).exists() {
                print_error("找不到配置文件 config.toml");
                return Ok(());
            }

            let config: Config = toml::from_str(&fs::read_to_string(config_path)?)?;

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

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                send_multi_emails(&email_tree, config, password, from, to).await?;
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
    use std::fs;

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

    // 测试生成钓鱼邮件
    #[test]
    fn test_generate_phishing_emails() -> Result<(), Box<dyn Error>> {
        let (db, tree) = setup_test_db()?;
        let template = create_test_template()?;
        
        // 确保generate目录不存在
        let _ = fs::remove_dir_all("./generate");
        
        generate_phishing_emails(&tree, template.path().to_str().unwrap())?;
        
        // 验证生成的文件
        let generated_file = fs::read_to_string("./generate/test1@example.com.html")?;
        assert!(generated_file.contains("ID: test1"));
        
        // 清理
        fs::remove_dir_all("./generate")?;
        fs::remove_dir_all("test_email_database")?;
        Ok(())
    }

    // 测试发送钓鱼邮件
    #[tokio::test]
    async fn test_send_phishing_emails() -> Result<(), Box<dyn Error>> {
        let (db, tree) = setup_test_db()?;
        let config_file = create_test_config()?;
        
        let config: Config = toml::from_str(&fs::read_to_string(config_file.path())?)?;
        
        // 使用无效的凭证测试（应该返回错误）
        let result = send_phishing_emails(&tree, config, "invalid_password".to_string()).await;
        assert!(result.is_err());
        
        // 清理
        fs::remove_dir_all("test_email_database")?;
        Ok(())
    }

    // 测试配置文件解析
    #[test]
    fn test_config_parsing() -> Result<(), Box<dyn Error>> {
        let config_file = create_test_config()?;
        let config: Config = toml::from_str(&fs::read_to_string(config_file.path())?)?;
        
        assert_eq!(config.smtp.server, "smtp.126.com");
        assert_eq!(config.smtp.username, "test@126.com");
        assert_eq!(config.smtp.interval, 1);
        assert_eq!(config.email.template, "template.html");
        
        Ok(())
    }

    // 添加显示邮箱列表的测试
    #[test]
    fn test_show_all_emails() -> Result<(), Box<dyn Error>> {
        let (db, tree) = setup_test_db()?;
        
        // 添加多个测试邮箱
        let entries = vec![
            db::EmailEntry {
                id: "test1".to_string(),
                email: "test1@example.com".to_string(),
            },
            db::EmailEntry {
                id: "test2".to_string(),
                email: "test2@example.com".to_string(),
            },
        ];
        
        for entry in &entries {
            tree.insert(entry.id.as_bytes(), bincode::serialize(entry)?)?;
        }
        
        show_all_emails(&tree)?;
        
        // 清理
        fs::remove_dir_all("test_email_database")?;
        Ok(())
    }
}
