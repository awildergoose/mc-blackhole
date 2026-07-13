use crate::{
    codecs::base::{MCDecode, MCEncode},
    proto::{packet_bytes::PacketBytes, varint::VarInt},
    world::entity::player::Item,
};

// incomplete
#[derive(Debug, Clone)]
pub struct ItemStack {
    pub count: VarInt,
    pub item: Option<Item>,
}

impl ItemStack {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            count: 0,
            item: None,
        }
    }
}

impl MCEncode for ItemStack {
    fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()> {
        dst.put_var_int(self.count)?;

        if self.count > 0 {
            dst.put_var_int(
                self.item
                    .ok_or_else(|| anyhow::anyhow!("count != 0 but item is None"))?,
            )?;
        }

        Ok(())
    }
}

impl MCDecode for ItemStack {
    fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
        let count = src.get_var_int()?;
        let item = if count > 0 {
            Some(src.get_var_int()?)
        } else {
            None
        };

        Ok(Self { count, item })
    }
}
