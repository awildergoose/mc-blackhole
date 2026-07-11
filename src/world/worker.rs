use cgmath::Vector3;
use tokio::sync::{mpsc, oneshot};

use crate::world::{
    entity::player::PlayerEntity,
    level::{EntityId, Level},
};

pub enum WorldRequest {
    GetViewDistance {
        respond: oneshot::Sender<i32>,
    },
    GetPlayerPosition {
        player: EntityId,
        respond: oneshot::Sender<Vector3<f64>>,
    },

    UpdatePlayerPosition {
        player: EntityId,
        position: Vector3<f64>,
    },
    AddMetaball {
        position: Vector3<i32>,
        perma: bool,
    },
    Stop,
    Tick,
}

#[derive(Clone)]
pub struct WorldHandle {
    tx: mpsc::Sender<WorldRequest>,
}

impl WorldHandle {
    pub async fn get_view_distance(&self) -> anyhow::Result<i32> {
        let (tx, rx) = oneshot::channel();
        self.send(WorldRequest::GetViewDistance { respond: tx })
            .await?;
        Ok(rx.await?)
    }

    pub async fn get_player_position(&self, player: EntityId) -> anyhow::Result<Vector3<f64>> {
        let (tx, rx) = oneshot::channel();
        self.send(WorldRequest::GetPlayerPosition {
            player,
            respond: tx,
        })
        .await?;
        Ok(rx.await?)
    }

    pub async fn send(&self, request: WorldRequest) -> anyhow::Result<()> {
        self.tx.send(request).await?;
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
                WorldRequest::GetPlayerPosition { player, respond } => {
                    let pos = self
                        .level
                        .with_entity::<PlayerEntity, _, _>(player, |_level, player| {
                            player.get_position()
                        })
                        .await?;
                    let _ = respond.send(pos);
                }
                WorldRequest::UpdatePlayerPosition { player, position } => {
                    self.level
                        .with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
                            p.update_position(position);
                        })
                        .await?;
                }
                WorldRequest::AddMetaball { position, perma } => {
                    self.level
                        .add_metaball(
                            position,
                            2.0,
                            2.0,
                            super::palette::PaletteBlockKind::OakPlanks,
                            perma,
                        )
                        .await?;
                }
                WorldRequest::Stop => {
                    break;
                }
                WorldRequest::Tick => {
                    self.level.tick().await?;
                }
            }
        }

        self.rx.close();

        Ok(())
    }
}
