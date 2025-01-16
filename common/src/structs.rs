use sled;
use {
    byteorder::BigEndian,
    zerocopy::{
        AsBytes, LayoutVerified, U16, U32, FromBytes, Unaligned
    },
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::fs;

/*
atype
0 -- 点击邮件
1 -- 打开链接
2 -- 提交数据
*/
#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
pub struct Action {
    pub id: U16<BigEndian>,
    pub user_id: [u8; 4],
    pub time: [u8; 32],
    pub ip: [u8; 32],
    pub atype: U16<BigEndian>,
    pub data_id: U16<BigEndian>,
}

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
pub struct Data {
    pub id: U16<BigEndian>,
    pub data: [u8; 512],
}

pub struct ActionTree(pub Arc<sled::Tree>);

impl ActionTree {
    pub fn get_tree(&self) -> &Arc<sled::Tree> {
        &self.0
    }
    pub fn clone_tree(&self) -> Arc<sled::Tree> {
        Arc::clone(&self.0)
    }
}


pub struct DataTree(pub Arc<sled::Tree>);

impl DataTree {
    pub fn get_tree(&self) -> &Arc<sled::Tree> {
        &self.0
    }
    pub fn clone_tree(&self) -> Arc<sled::Tree> {
        Arc::clone(&self.0)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EmailEntry {
    pub id: String,
    pub email: String,
}

pub struct EmailTree(pub Arc<sled::Tree>);

impl EmailTree {
    pub fn get_tree(&self) -> &Arc<sled::Tree> {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: Server,
    // pub routes: Routes,
    pub paths: Paths,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub ip: String,
    pub port: u16,
}

// #[derive(Debug, Clone, Deserialize)]
// pub struct Routes {
//     pub submit: String,
//     pub index: String,
//     pub image: String,
// }

#[derive(Debug, Clone, Deserialize)]
pub struct Paths {
    pub phish_page: String,
    pub redirect_url: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string("config.toml")?;
        Ok(toml::from_str(&content)?)
    }
} 