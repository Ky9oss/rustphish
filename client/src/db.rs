use bincode;
use colored::*;
use shared::structs::{Action, Data};
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use zerocopy::LayoutVerified;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailEntry {
    pub id: String,
    pub email: String,
}

fn generate_random_id() -> String {
    let mut rng = thread_rng();
    let id: String = (0..4)
        .map(|_| {
            if rng.gen_bool(0.5) {
                // 50%概率生成数字
                (b'0' + rng.gen_range(0..10)) as char
            } else {
                // 50%概率生成字母
                rng.sample(Alphanumeric) as char
            }
        })
        .collect();
    id
}

pub fn load_emails_to_db(tree: &sled::Tree, file_path: &str) -> Result<(), Box<dyn Error>> {
    // 读取文件内容
    let content = fs::read_to_string(file_path)?;

    // 获取现有邮箱列表
    let existing_emails: Vec<String> = get_all_emails(tree)?
        .into_iter()
        .map(|entry| entry.email)
        .collect();

    let mut added_count = 0;
    let mut skipped_count = 0;

    // 处理每一行
    for email in content.lines() {
        let email = email.trim();
        if !email.is_empty() {
            // 检查邮箱是否已存在
            if existing_emails.contains(&email.to_string()) {
                print_info(&format!("跳过已存在的邮箱: {}", email));
                skipped_count += 1;
                continue;
            }

            // 生成唯一ID
            let mut id;
            loop {
                id = generate_random_id();
                // 检查ID是否已存在
                if tree.get(id.as_bytes())?.is_none() {
                    break;
                }
            }

            let entry = EmailEntry {
                id: id.clone(),
                email: email.to_string(),
            };

            // 序列化并存储
            let serialized = bincode::serialize(&entry)?;
            tree.insert(id.as_bytes(), serialized)?;
            print_success(&format!("添加新邮箱: {} (ID: {})", email, id));
            added_count += 1;
        }
    }

    // 确保数据写入磁盘
    tree.flush()?;

    // 打印统计信息
    if added_count > 0 {
        print_success(&format!("成功添加 {} 个新邮箱", added_count));
    }
    if skipped_count > 0 {
        print_info(&format!("跳过 {} 个重复邮箱", skipped_count));
    }
    if added_count == 0 && skipped_count == 0 {
        print_info("没有找到有效的邮箱地址");
    }

    Ok(())
}

pub fn get_all_emails(email_tree: &sled::Tree) -> Result<Vec<EmailEntry>, Box<dyn Error>> {
    let mut emails = Vec::new();

    for result in email_tree.iter() {
        let (key, value) = result?;

        if key == "max_id".as_bytes() {
            continue;
        }

        if let Ok(entry) = bincode::deserialize::<EmailEntry>(&value) {
            emails.push(entry);
        }
    }

    emails.sort_by_key(|entry| entry.id.clone());

    Ok(emails)
}

pub fn format_phishing_record(
    action: &Action,
    data_tree: &sled::Tree,
    email_tree: &sled::Tree,
    next_key: Option<Vec<u8>>,
) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // 1. 获取email
    let user_id = std::str::from_utf8(&action.user_id)?.trim_end_matches('\0');
    let email = match email_tree.get(user_id.as_bytes())? {
        Some(email_bytes) => {
            let email_entry: EmailEntry = bincode::deserialize(&email_bytes)?;
            email_entry.email
        }
        None => format!("未知邮箱 (ID: {})", user_id),
    };

    // 2. 获取时间和IP
    let time = shared::utils::u8_32_to_string_gbk(action.time);
    let ip = shared::utils::u8_32_to_string_gbk(action.ip);

    // 3. 根据atype格式化输出
    match action.atype.get() {
        0 => {
            println!(
                "[{}] {}（email：{}，ip：{}）",
                time,
                "打开邮件".green(),
                email,
                ip,
            );
        }
        1 => {
            println!(
                "[{}] {}（email：{}，ip：{}）",
                time,
                "点击链接".yellow(),
                email,
                ip,
            );
        }
        2 => {
            // 如果data_id不为0，查找对应的数据
            let data_content = if action.data_id.get() != 0 {
                if let Ok(Some(data_bytes)) =
                    data_tree.get(format!("data-{}", action.data_id.get()))
                {
                    if let Some((data, _)) =
                        LayoutVerified::<&[u8], Data>::new_from_prefix(&data_bytes)
                    {
                        shared::utils::u8_512_to_string_gbk(data.data)
                    } else {
                        "数据格式错误".to_string()
                    }
                } else {
                    "数据未找到".to_string()
                }
            } else {
                "无数据".to_string()
            };

            println!(
                "[{}] {}（email：{}，ip：{}）（data: {}）",
                time,
                "提交数据".red(),
                email,
                ip,
                data_content
            );
        }
        3 => {
            println!(
                "[{}] {}（email：{}，ip：{}）",
                time,
                "点击木马".red(),
                email,
                ip,
            );
        }
        _ => {
            println!(
                "[{}] {}（atype: {}）（email：{}，ip：{}）",
                time,
                "未知行为".yellow(),
                action.atype.get(),
                email,
                ip
            );
        }
    }

    Ok(next_key)
}

pub fn traverse_actions(
    action_tree: &sled::Tree,
    data_tree: &sled::Tree,
    email_tree: &sled::Tree,
) -> Result<(), Box<dyn Error>> {
    print_info("开始遍历钓鱼记录");

    let mut current_key = action_tree.get_lt(vec![255])?.map(|(k, _)| k);
    let mut count = 0;
    let mut error_count = 0;

    while let Some(ref key) = current_key {
        count += 1;

        if let Some(value) = action_tree.get(key)? {
            // 检查value的长度是否符合Action结构体的大小
            if value.len() < std::mem::size_of::<Action>() {
                error_count += 1;
                if error_count <= 5 {
                    // 只显示前5个错误
                    print_error(&format!(
                        "记录 {} 数据格式错误: 期望长度 >= {}, 实际长度 {}",
                        count,
                        std::mem::size_of::<Action>(),
                        value.len()
                    ));
                }
                // 继续处理下一条记录
                current_key = action_tree.get_lt(key)?.map(|(k, _)| k);
                continue;
            }

            if let Some((action, _)) = LayoutVerified::<&[u8], Action>::new_from_prefix(&value) {
                let next_key = action_tree.get_lt(key)?.map(|(k, _)| k);

                match format_phishing_record(
                    &action,
                    data_tree,
                    email_tree,
                    next_key.clone().map(|k| k.to_vec()),
                ) {
                    Ok(Some(k)) => current_key = Some(k.into()),
                    Ok(None) => break,
                    Err(e) => {
                        error_count += 1;
                        if error_count <= 5 {
                            print_error(&format!("处理记录 {} 时出错: {}", count, e));
                        }
                        current_key = next_key;
                    }
                }
            } else {
                error_count += 1;
                if error_count <= 5 {
                    print_error(&format!("记录 {} 数据解析失败: {:?}", count, value));
                }
                current_key = action_tree.get_lt(key)?.map(|(k, _)| k);
            }
        } else {
            break;
        }
    }

    if error_count > 0 {
        print_error(&format!("共发现 {} 条错误记录", error_count));
    }
    print_success(&format!("遍历完成，共处理 {} 条记录", count));
    Ok(())
}

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

pub fn delete_email_by_id(tree: &sled::Tree, target_id: &str) -> Result<(), Box<dyn Error>> {
    // 检查ID是否存在
    match tree.get(target_id.as_bytes())? {
        Some(value) => {
            // 反序列化以获取邮箱信息（用于显示）
            let entry: EmailEntry = bincode::deserialize(&value)?;

            // 删除记录
            tree.remove(target_id.as_bytes())?;
            tree.flush()?;

            print_success(&format!("成功删除邮箱: {} (ID: {})", entry.email, entry.id));
            Ok(())
        }
        None => {
            print_error(&format!("未找到ID为 {} 的记录", target_id));
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "ID not found",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // 辅助函数：创建临时邮件列表文件
    fn create_test_email_file() -> Result<NamedTempFile, Box<dyn Error>> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "test1@example.com")?;
        writeln!(file, "test2@example.com")?;
        writeln!(file, "test3@example.com")?;
        Ok(file)
    }

    #[test]
    fn test_generate_random_id() {
        let id = generate_random_id();
        assert_eq!(id.len(), 4);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_load_emails_to_db() -> Result<(), Box<dyn Error>> {
        let db = sled::open("test_email_database")?;
        let tree = db.open_tree("test_emails")?;

        let email_file = create_test_email_file()?;

        load_emails_to_db(&tree, email_file.path().to_str().unwrap())?;

        let emails = get_all_emails(&tree)?;
        assert_eq!(emails.len(), 3);

        // 清理测试数据库
        fs::remove_dir_all("test_email_database")?;
        Ok(())
    }

    #[test]
    fn test_get_all_emails() -> Result<(), Box<dyn Error>> {
        let db = sled::open("test_email_database2")?;
        let tree = db.open_tree("test_emails")?;

        // 插入测试数据
        let test_entries = vec![
            EmailEntry {
                id: "test1".to_string(),
                email: "test1@example.com".to_string(),
            },
            EmailEntry {
                id: "test2".to_string(),
                email: "test2@example.com".to_string(),
            },
        ];

        for entry in &test_entries {
            tree.insert(entry.id.as_bytes(), bincode::serialize(entry)?)?;
        }

        let retrieved_emails = get_all_emails(&tree)?;
        assert_eq!(retrieved_emails.len(), test_entries.len());

        // 清理测试数据库
        fs::remove_dir_all("test_email_database2")?;
        Ok(())
    }

    #[test]
    fn test_delete_email_by_id() -> Result<(), Box<dyn Error>> {
        let db = sled::open("test_email_database3")?;
        let tree = db.open_tree("test_emails")?;

        // 添加测试数据
        let test_entry = EmailEntry {
            id: "test1".to_string(),
            email: "test1@example.com".to_string(),
        };

        tree.insert(test_entry.id.as_bytes(), bincode::serialize(&test_entry)?)?;

        // 测试删除存在的ID
        assert!(delete_email_by_id(&tree, "test1").is_ok());

        // 测试删除不存在的ID
        assert!(delete_email_by_id(&tree, "nonexistent").is_err());

        // 清理
        fs::remove_dir_all("test_email_database3")?;
        Ok(())
    }
}
