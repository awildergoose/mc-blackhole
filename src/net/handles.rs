use tokio::sync::mpsc;

use crate::proto::packets::Packet;

pub enum PacketWriterMessage {
    SetCompression(i32),
    Write(i32, Vec<u8>),
}

#[derive(Clone)]
pub struct PacketWriterHandle {
    tx: mpsc::Sender<PacketWriterMessage>,
}

impl PacketWriterHandle {
    pub async fn write_pkt<T>(&self, pkt: T) -> anyhow::Result<()>
    where
        T: Packet,
    {
        self.write_packet(T::ID, pkt.encoded()?.to_vec()).await
    }

    pub async fn write_packet(&self, packet_id: i32, body: Vec<u8>) -> anyhow::Result<()> {
        self.tx
            .send(PacketWriterMessage::Write(packet_id, body))
            .await?;
        Ok(())
    }

    pub async fn set_compression(&self, threshold: i32) -> anyhow::Result<()> {
        self.tx
            .send(PacketWriterMessage::SetCompression(threshold))
            .await?;
        Ok(())
    }
}

impl From<mpsc::Sender<PacketWriterMessage>> for PacketWriterHandle {
    fn from(tx: mpsc::Sender<PacketWriterMessage>) -> Self {
        Self { tx }
    }
}
