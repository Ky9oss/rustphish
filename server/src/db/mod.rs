use sled;
use {
    zerocopy::{
        AsBytes, LayoutVerified
    },
};
use common::structs::{Action, Data};



pub fn get_next_id_for_tree(tree: &sled::Tree) -> sled::Result<u16> {
    let id_key = "next_id";
    
    let new_id = tree.update_and_fetch(id_key, |id_opt| {
        let next_id = match id_opt {
            Some(id_bytes) => {
                if id_bytes.len() != 2 {
                    return None;  // 无效的ID格式
                }
                let mut id_arr = [0; 2];
                id_arr.copy_from_slice(id_bytes);
                u16::from_be_bytes(id_arr).checked_add(1)?
            },
            None => 1,
        };
        Some(next_id.to_be_bytes().to_vec())
    })?;

    new_id.ok_or_else(|| {
        sled::Error::Unsupported(String::from("ID generation failed or would overflow"))
    }).and_then(|id_bytes| {
        if id_bytes.len() != 2 {
            return Err(sled::Error::Unsupported(String::from("Invalid ID format")));
        }
        let mut id_arr = [0; 2];
        id_arr.copy_from_slice(&id_bytes);
        Ok(u16::from_be_bytes(id_arr))
    })
}

pub fn create_action(action_tree: &sled::Tree, action: Action) -> sled::Result<()> {
    let action_key = format!("action-{}", action.id);
    action_tree.insert(action_key, action.as_bytes())?;
    action_tree.flush()?;
    Ok(())
}

pub fn create_data(data_tree: &sled::Tree, data: Data) -> sled::Result<()> {
    let data_key = format!("data-{}", data.id);
    data_tree.insert(data_key, data.as_bytes())?;
    data_tree.flush()?;
    Ok(())
}

pub fn remove_action(action_tree: &sled::Tree, id: u16) -> sled::Result<()>{
    let action_key = format!("action-{}", id);
    action_tree.remove(&action_key)?;
    Ok(())
}

pub fn get_all_actions(action_tree: &sled::Tree) -> sled::Result<Vec<Action>> {
    let mut actions = Vec::new();
    for result in action_tree.iter() {
        let (_, value_bytes) = result?;
        if let Some((action, _)) = LayoutVerified::<&[u8], Action>::new_from_prefix(&*value_bytes) {
            actions.push(action.clone());
        }
    }
    Ok(actions)
}

pub fn get_all_datas(data_tree: &sled::Tree) -> sled::Result<Vec<Data>> {
    let mut datas = Vec::new();
    for result in data_tree.iter() {
        let (_, value_bytes) = result?;
        if let Some((data, _)) = LayoutVerified::<&[u8], Data>::new_from_prefix(&*value_bytes) {
            datas.push(data.clone());
        }
    }
    Ok(datas)
}
