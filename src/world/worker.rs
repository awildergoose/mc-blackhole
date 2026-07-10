use cgmath::Vector3;
use tokio::sync::{mpsc, oneshot};

use crate::{
    net::handles::PacketWriterHandle,
    world::level::{EntityId, Level},
};

pub enum WorldRequest {
    GetViewDistance {
        respond: oneshot::Sender<i32>,
    },
    UpdatePlayerPosition {
        player: EntityId,
        position: Vector3<f64>,
        packet_writer: PacketWriterHandle,
    },
    Stop,
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
        packet_writer: PacketWriterHandle,
    ) -> anyhow::Result<()> {
        self.tx
            .send(WorldRequest::UpdatePlayerPosition {
                player,
                position,
                packet_writer,
            })
            .await?;
        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        self.tx.send(WorldRequest::Stop).await?;
        Ok(())
    }
}

pub struct WorldWorker {
    level: Option<Level>,
    rx: mpsc::Receiver<WorldRequest>,
}

impl WorldWorker {
    #[must_use]
    pub fn new(level: Level) -> (Self, WorldHandle) {
        let (tx, rx) = mpsc::channel(256);

        (
            Self {
                level: Some(level),
                rx,
            },
            WorldHandle { tx },
        )
    }

    const fn level(&mut self) -> &mut Level {
        self.level
            .as_mut()
            .expect("tried to get level post-closing channel?")
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while let Some(request) = self.rx.recv().await {
            match request {
                WorldRequest::GetViewDistance { respond } => {
                    let _ = respond.send(self.level().view_distance);
                }
                WorldRequest::UpdatePlayerPosition {
                    player,
                    position,
                    packet_writer,
                } => {
                    self.level()
                        .update_player_position(player, position, packet_writer)
                        .await?;
                }
                WorldRequest::Stop => {
                    println!("World received a stop signal, we're ending it!");
                    break;
                }
            }
        }

        self.rx.close();
        self.level = None;

        Ok(())
    }
}
