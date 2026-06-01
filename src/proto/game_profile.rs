use anyhow::bail;
use uuid::Uuid;

use crate::proto::packet_bytes::PacketBytes;

const USERNAME_MAX: usize = 16;

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub uuid: Uuid,
    pub username: String,
}

impl GameProfile {
    pub fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()> {
        dst.put_uuid(self.uuid)?;

        if self.username.len() > USERNAME_MAX {
            bail!("username too long");
        }
        dst.put_str(&self.username)?;

        // properties
        dst.put_var_int(0)?;

        Ok(())
    }

    pub fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
        let uuid = src.get_uuid()?;
        let username = src.get_string()?;
        // ignore props

        Ok(Self { uuid, username })
    }
}
