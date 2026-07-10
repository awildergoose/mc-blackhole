use tokio::sync::mpsc;

use crate::{handlers::PacketWriterChannel, proto::packets::Packet};

#[derive(Clone)]
pub struct PacketWriterHandle {
    tx: mpsc::Sender<PacketWriterChannel>,
}

impl PacketWriterHandle {
    pub async fn write_pkt<T>(&self, pkt: T) -> anyhow::Result<()>
    where
        T: Packet,
    {
        self.write_packet(T::ID, pkt.encoded()?.to_vec()).await
    }

    pub async fn write_packet(&self, packet_id: i32, body: Vec<u8>) -> anyhow::Result<()> {
        self.tx.send((packet_id, body)).await?;
        Ok(())
    }
}

impl From<mpsc::Sender<PacketWriterChannel>> for PacketWriterHandle {
    fn from(tx: mpsc::Sender<PacketWriterChannel>) -> Self {
        Self { tx }
    }
}
