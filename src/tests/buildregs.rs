//! Not really a test but whatever gets the job done lol
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegistryEntry {
    protocol_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Registry {
    default: Option<String>,
    entries: HashMap<String, RegistryEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockState {
    id: i64,
    default: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockEntry {
    states: Vec<BlockState>,
}

#[test]
fn main() {
    let regs = std::fs::read_to_string("generated/registries.json").unwrap();
    let regs: HashMap<String, Registry> = serde_json::from_str(&regs).unwrap();
    let blocks_reg = std::fs::read_to_string("generated/blocks.json").unwrap();
    let blocks_reg: HashMap<String, BlockEntry> = serde_json::from_str(&blocks_reg).unwrap();
    let items = &regs.get("minecraft:item").unwrap().entries;

    let mut states = HashMap::new();

    for block in blocks_reg {
        states.insert(
            block.0,
            block
                .1
                .states
                .iter()
                .find(|s| s.default.is_some_and(|s| s))
                .unwrap()
                .id,
        );
    }

    let mut generated = String::new();
    generated += "pub static ITEM_TO_BLOCK: std::sync::LazyLock<std::collections::HashMap<crate::world::entity::player::Item, i32>> = std::sync::LazyLock::new(|| {
    let mut map = std::collections::HashMap::new();\n";

    // we need to map from item to block
    for item in items {
        // is there an entry of this item in the blocks registry?
        let block = states.get(item.0);

        if let Some(block) = block {
            let _ = writeln!(
                generated,
                "    map.insert({}, {});",
                item.1.protocol_id, block
            );
        }
    }

    generated += "    map
});";

    std::fs::write("src/generated/registries.rs", generated).unwrap();
}
