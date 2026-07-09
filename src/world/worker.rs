use cgmath::Vector3;
use tokio::sync::{mpsc, oneshot};

use crate::world::level::{EntityId, Level, UPPPacket};

pub enum WorldRequest {
    GetViewDistance {
        respond: oneshot::Sender<i32>,
    },
    UpdatePlayerPosition {
        player: EntityId,
        position: Vector3<f64>,
        upppacket_tx: mpsc::Sender<UPPPacket>,
    },
}

#[derive(Clone)]
pub struct WorldHandle {
    tx: mpsc::Sender<WorldRequest>,
}

impl WorldHandle {
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
        upppacket_tx: mpsc::Sender<UPPPacket>,
    ) -> anyhow::Result<()> {
        self.tx
            .send(WorldRequest::UpdatePlayerPosition {
                player,
                position,
                upppacket_tx,
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
                WorldRequest::GetViewDistance { respond } => {
                    let _ = respond.send(self.level.view_distance);
                }
                WorldRequest::UpdatePlayerPosition {
                    player,
                    position,
                    upppacket_tx,
                } => {
                    self.level
                        .update_player_position(player, position, upppacket_tx)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
