use anyhow::bail;
use uuid::Uuid;

use crate::{
    codecs::base::{MCDecode, MCEncode},
    proto::packet_bytes::PacketBytes,
};

const USERNAME_MAX: usize = 16;

// TODO: Migrate this into the codecs system
#[derive(Debug, Clone)]
pub struct GameProfile {
    pub uuid: Uuid,
    pub username: String,
}

impl GameProfile {
    #[must_use]
    pub const fn new(uuid: Uuid, username: String) -> Self {
        Self { uuid, username }
    }
}

impl MCEncode for GameProfile {
    fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()> {
        dst.put_uuid(self.uuid)?;

        if self.username.len() > USERNAME_MAX {
            bail!("username too long");
        }
        dst.put_str(&self.username)?;

        // properties
        dst.put_var_int(0)?;

        Ok(())
    }
}

impl MCDecode for GameProfile {
    fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
        let uuid = src.get_uuid()?;
        let username = src.get_string()?;
        // ignore props

        Ok(Self { uuid, username })
    }
}
