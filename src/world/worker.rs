use cgmath::Vector3;
use tokio::sync::{mpsc, oneshot};

use crate::{
    proto::packets::play::GameMode,
    world::{
        entity::{player::PlayerEntity, structure::StructureEntity},
        level::{EntityId, Level},
    },
};

pub enum WorldRequest {
    GetViewDistance {
        respond: oneshot::Sender<i32>,
    },
    GetPlayerPosition {
        player: EntityId,
        respond: oneshot::Sender<Vector3<f64>>,
    },
    GetPlayerFlying {
        player: EntityId,
        respond: oneshot::Sender<bool>,
    },
    GetPlayerGameMode {
        player: EntityId,
        respond: oneshot::Sender<GameMode>,
    },
    GetWorldSpawnPosition {
        respond: oneshot::Sender<Vector3<f64>>,
    },
    GetDiggerPosition {
        digger: EntityId,
        respond: oneshot::Sender<Vector3<i32>>,
    },
    AddPlayer {
        player: PlayerEntity,
        respond: oneshot::Sender<EntityId>,
    },

    /// To be used by commands
    SetPlayerPosition {
        player: EntityId,
        position: Vector3<f64>,
    },
    /// To be used when the player moves regularly
    PlayerMove {
        player: EntityId,
        position: Vector3<f64>,
    },
    UpdatePlayerFlying {
        player: EntityId,
        is_flying: bool,
    },
    UpdatePlayerGameMode {
        player: EntityId,
        game_mode: GameMode,
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

macro_rules! makegetalias {
    ($name:expr, $ret:ty, $($arg:expr => $argty:ty),*) => {
        paste::paste! {
            pub async fn [<$name:snake>](&self, $($arg: $argty),*) -> anyhow::Result<$ret> {
                let (tx, rx) = oneshot::channel();
                self.send(WorldRequest::$name {
                    respond: tx,
                    $($arg),*
                })
                    .await?;
                Ok(rx.await?)
            }
        }
    };
    ($name:expr, $ret:ty) => {
        paste::paste! {
            pub async fn [<$name:snake>](&self) -> anyhow::Result<$ret> {
                let (tx, rx) = oneshot::channel();
                self.send(WorldRequest::$name {
                    respond: tx,
                })
                    .await?;
                Ok(rx.await?)
            }
        }
    };
}

impl WorldHandle {
    makegetalias!(GetViewDistance, i32);
    makegetalias!(GetWorldSpawnPosition, Vector3<f64>);
    makegetalias!(GetPlayerPosition, Vector3<f64>, player => EntityId);
    makegetalias!(GetPlayerFlying, bool, player => EntityId);
    makegetalias!(GetPlayerGameMode, GameMode, player => EntityId);
    makegetalias!(GetDiggerPosition, Vector3<i32>, digger => EntityId);
    makegetalias!(AddPlayer, EntityId, player => PlayerEntity);

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
                    let _ = respond.send(
                        self.level
                            .with_entity::<PlayerEntity, _, _>(player, |_level, player| {
                                player.position
                            })
                            .await?,
                    );
                }
                WorldRequest::GetPlayerFlying { player, respond } => {
                    let _ = respond.send(
                        self.level
                            .with_entity::<PlayerEntity, _, _>(player, |_level, player| {
                                player.flying
                            })
                            .await?,
                    );
                }
                WorldRequest::GetPlayerGameMode { player, respond } => {
                    let _ = respond.send(
                        self.level
                            .with_entity::<PlayerEntity, _, _>(player, |_level, player| {
                                player.game_mode
                            })
                            .await?,
                    );
                }
                WorldRequest::GetWorldSpawnPosition { respond } => {
                    let _ = respond.send(self.level.get_spawn_position());
                }
                WorldRequest::GetDiggerPosition { digger, respond } => {
                    let _ = respond.send(
                        self.level
                            .with_entity::<StructureEntity, _, _>(digger, |_level, digger| {
                                digger.position
                            })
                            .await?,
                    );
                }
                WorldRequest::AddPlayer { player, respond } => {
                    let _ = respond.send(self.level.add_player(player));
                }
                WorldRequest::SetPlayerPosition { player, position } => {
                    self.level
                        .with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
                            p.position = position;
                            p.prev_position = position;
                        })
                        .await?;
                }
                WorldRequest::PlayerMove { player, position } => {
                    self.level
                        .with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
                            p.position = position;
                        })
                        .await?;
                }
                WorldRequest::UpdatePlayerFlying { player, is_flying } => {
                    self.level
                        .with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
                            p.flying = is_flying;
                        })
                        .await?;
                }
                WorldRequest::UpdatePlayerGameMode { player, game_mode } => {
                    self.level
                        .with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
                            p.game_mode = game_mode;
                        })
                        .await?;
                }
                WorldRequest::AddMetaball { position, perma } => {
                    self.level
                        .add_metaball(
                            position,
                            6.0,
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
