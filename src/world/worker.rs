use cgmath::Vector3;
use tokio::sync::{mpsc, oneshot};

use crate::world::{
    chunk::Chunk,
    level::{ChunkPos, EntityId, Level, UPPPacket},
    palette::PaletteBlockKind,
};

pub enum WorldRequest {
    GetChunk {
        pos: ChunkPos,
        respond: oneshot::Sender<Option<Chunk>>,
    },
    GenerateChunk {
        pos: ChunkPos,
        respond: oneshot::Sender<Chunk>,
    },
    GetBlock {
        pos: Vector3<i32>,
        respond: oneshot::Sender<PaletteBlockKind>,
    },
    SetBlock {
        pos: Vector3<i32>,
        kind: PaletteBlockKind,
    },
    GetViewDistance {
        respond: oneshot::Sender<i32>,
    },
    UpdatePlayerPosition {
        player: EntityId,
        position: Vector3<f64>,
        packet_sender: mpsc::Sender<UPPPacket>,
    },
}

#[derive(Clone)]
pub struct WorldHandle {
    tx: mpsc::Sender<WorldRequest>,
}

impl WorldHandle {
    pub async fn get_block(&self, pos: Vector3<i32>) -> anyhow::Result<PaletteBlockKind> {
        let (tx, rx) = oneshot::channel();

        self.tx
            .send(WorldRequest::GetBlock { pos, respond: tx })
            .await?;

        Ok(rx.await?)
    }

    pub async fn set_block(&self, pos: Vector3<i32>, kind: PaletteBlockKind) -> anyhow::Result<()> {
        self.tx.send(WorldRequest::SetBlock { pos, kind }).await?;
        Ok(())
    }

    pub async fn get_chunk(&self, pos: ChunkPos) -> anyhow::Result<Chunk> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldRequest::GenerateChunk { pos, respond: tx })
            .await?;

        Ok(rx.await?)
    }

    pub async fn get_view_distance(&self) -> anyhow::Result<i32> {
        let (tx, rx) = oneshot::channel();

        self.tx
            .send(WorldRequest::GetViewDistance { respond: tx })
            .await?;

        Ok(rx.await?)
    }

    pub async fn update_player_position(
        &self,
        player: EntityId,
        position: Vector3<f64>,
        packet_sender: mpsc::Sender<UPPPacket>,
    ) -> anyhow::Result<()> {
        self.tx
            .send(WorldRequest::UpdatePlayerPosition {
                player,
                position,
                packet_sender,
            })
            .await?;
        Ok(())
    }
}

pub struct WorldWorker {
    level: Level,
    rx: mpsc::Receiver<WorldRequest>,
}

impl WorldWorker {
    #[must_use]
    pub fn new(level: Level) -> (Self, WorldHandle) {
        let (tx, rx) = mpsc::channel(256);

        (Self { level, rx }, WorldHandle { tx })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while let Some(request) = self.rx.recv().await {
            match request {
                WorldRequest::GetBlock { pos, respond } => {
                    let block = self.level.get_block(pos.x, pos.y, pos.z);

                    let _ = respond.send(block);
                }

                WorldRequest::SetBlock { pos, kind } => {
                    self.level.set_block(pos.x, pos.y, pos.z, kind);
                }

                WorldRequest::GenerateChunk { pos, respond } => {
                    let chunk = self.level.generate_chunk(pos);

                    let _ = respond.send(chunk);
                }

                WorldRequest::GetChunk { pos, respond } => {
                    let chunk = self.level.chunks.get(&pos).cloned();

                    let _ = respond.send(chunk);
                }

                WorldRequest::GetViewDistance { respond } => {
                    let _ = respond.send(self.level.view_distance);
                }
                WorldRequest::UpdatePlayerPosition {
                    player,
                    position,
                    packet_sender,
                } => {
                    self.level
                        .update_player_position(player, position, packet_sender)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
