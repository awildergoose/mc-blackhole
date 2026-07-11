use std::{sync::Arc, time::Duration};

use cgmath::Vector3;
use tokio::{runtime::Handle, sync::RwLock};

use crate::{
    codecs::{base::MCDecode, game_profile::GameProfile},
    expect_packet,
    net::{framing::FramedConnRead, handles::PacketWriterHandle},
    proto::{
        packet_bytes::PacketBytes,
        packets::{
            Packet,
            config::{
                KnownPack, cs_client_information::CsClientInformation,
                cs_config_custom_payload::CsConfigCustomPayload,
                cs_finish_configuration::CsFinishConfiguration,
                cs_select_known_packs::CsSelectKnownPacks,
                sc_finish_configuration::ScFinishConfiguration, sc_registries::ScRegistries,
                sc_select_known_packs::ScSelectKnownPacks, sc_tags::ScTags,
                sc_update_enabled_features::ScUpdateEnabledFeatures,
            },
            handshake::cs_intention::CsIntention,
            login::{
                cs_login_acknowledged::CsLoginAcknowledged, cs_login_start::CsLoginStart,
                sc_login_success::ScLoginSuccess, sc_set_compression::ScSetCompression,
            },
            play::{
                ByteGameMode, EntityEvent, GameEvent, GameMode,
                cs_accept_teleportation::CsAcceptTeleportation,
                cs_change_game_mode::CsChangeGameMode, cs_chat_command::CsChatCommand,
                cs_chunk_batch_received::CsChunkBatchReceived, cs_client_tick_end::CsClientTickEnd,
                cs_custom_payload::CsCustomPayload, cs_keep_alive::CsKeepAlive,
                cs_move_player_pos::CsMovePlayerPos, cs_move_player_pos_rot::CsMovePlayerPosRot,
                cs_move_player_rot::CsMovePlayerRot, cs_ping_request::CsPingRequest,
                cs_player_abilities::CsPlayerAbilities, cs_player_action::CsPlayerAction,
                cs_player_command::CsPlayerCommand, cs_player_input::CsPlayerInput,
                cs_player_loaded::CsPlayerLoaded, cs_set_carried_item::CsSetCarriedItem,
                cs_swing::CsSwing, sc_entity_event::ScEntityEvent, sc_game_event::ScGameEvent,
                sc_keep_alive::ScKeepAlive, sc_login::ScLogin,
                sc_player_abilities::ScPlayerAbilities, sc_player_position::ScPlayerPosition,
                sc_plugin_message::ScPluginMessage, sc_set_center_chunk::ScSetCenterChunk,
            },
        },
        raw::{regs, tags},
    },
    world::{
        entity::player::PlayerEntity,
        level::Level,
        palette::PaletteBlockKind,
        worker::{WorldHandle, WorldRequest, WorldWorker},
    },
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum ConnectionState {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

struct StopWorldOnDrop(WorldHandle);

impl Drop for StopWorldOnDrop {
    fn drop(&mut self) {
        let handle = Handle::current();
        let world = self.0.clone();

        handle.spawn(async move {
            let _ = world.send(WorldRequest::Stop).await;
        });
    }
}

#[expect(clippy::too_many_lines)]
pub async fn handle_connection(
    mut rd: FramedConnRead,
    writer: PacketWriterHandle,
) -> anyhow::Result<()> {
    let state = Arc::new(RwLock::new(ConnectionState::Handshaking));
    let intention = expect_packet!(rd, CsIntention)?;
    if intention.intent != 2 {
        // not a login intention
        return Ok(());
    }
    let login = expect_packet!(rd, CsLoginStart)?;

    {
        *state.write().await = ConnectionState::Login;
    }

    let sc = ScSetCompression::new(256);
    writer.write_pkt(sc.clone()).await?;

    writer.set_compression(sc.threshold).await?;

    let ls = ScLoginSuccess::new(GameProfile::new(login.uuid, login.username));
    writer.write_pkt(ls.clone()).await?;

    let mut client_tick = 0;
    let mut body;
    let mut level = Level::new(16);
    let player = level.add_player(PlayerEntity::new(writer.clone()));

    level
        .add_metaball(
            Vector3::new(0, 23, 0),
            8.0,
            2.0,
            PaletteBlockKind::OakPlanks,
            false,
        )
        .await?;
    level
        .set_block_perma(0, 0, 0, PaletteBlockKind::Grass)
        .await?;

    let (worker, world) = WorldWorker::new(level);
    let _guard = StopWorldOnDrop(world.clone());

    let handle = tokio::spawn(async move {
        if let Err(e) = worker.run().await {
            eprintln!("world worker error: {e:?}");
        }
    });

    let ticker_world = world.clone();
    let ticker_state = state.clone();
    let ticker_handle = tokio::spawn(async move {
        // 20 TPS
        let mut interval = tokio::time::interval(Duration::from_millis(50));

        loop {
            interval.tick().await;
            if *ticker_state.read().await != ConnectionState::Play {
                continue;
            }
            if ticker_world.send(WorldRequest::Tick).await.is_err() {
                break;
            }
        }
    });

    loop {
        match rd.read_packet().await {
            Ok((id, mut data)) => {
                let current_state = *state.read().await;

                match current_state {
                    ConnectionState::Login => {
                        if id == CsLoginAcknowledged::ID {
                            writer
                                .write_pkt(ScUpdateEnabledFeatures {
                                    features: vec!["minecraft:vanilla".to_owned()],
                                })
                                .await?;
                            writer
                                .write_pkt(ScSelectKnownPacks {
                                    features: vec![KnownPack::new(
                                        "minecraft".to_owned(),
                                        "core".to_owned(),
                                        "1.21.10".to_owned(),
                                    )],
                                })
                                .await?;

                            {
                                *state.write().await = ConnectionState::Configuration;
                            }
                        } else {
                            eprintln!("login received packet id {id:X}");
                        }
                    }
                    ConnectionState::Configuration => {
                        if id == CsConfigCustomPayload::ID || id == CsSelectKnownPacks::ID {
                            continue;
                        }

                        if id == CsClientInformation::ID {
                            // registries
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION0)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION1)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION2)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION3)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION4)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION5)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION6)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION7)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION8)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION9)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION10)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION11)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION12)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION13)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION14)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION15)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION16)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION17)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION18)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION19)))
                                .await?;
                            writer
                                .write_pkt(ScRegistries::new(Vec::from(regs::SECTION20)))
                                .await?;
                            writer.write_pkt(ScTags::new(Vec::from(tags::TAGS))).await?;
                            writer.write_pkt(ScFinishConfiguration::new()).await?;
                        } else if id == CsFinishConfiguration::ID {
                            writer
                                .write_pkt(ScLogin {
                                    entity_id: 0,
                                    is_hardcore: false,
                                    dimensions: vec!["overworld".to_owned()],
                                    max_players: 1,
                                    view_distance: world.get_view_distance().await?,
                                    simulation_distance: world.get_view_distance().await?,
                                    reduced_debug_info: false,
                                    respawn_screen: false,
                                    limited_crafting: false,
                                    dimension_type: 0,
                                    dimension: "overworld".to_owned(),
                                    seed: 0,
                                    gamemode: ByteGameMode::Survival,
                                    prev_gamemode: 0xFF,
                                    is_debug: false,
                                    is_flat: false,
                                    has_death_location: false,
                                    portal_cooldown: 0,
                                    sea_level: 63,
                                    secure_chat: false,
                                })
                                .await?;

                            writer
                                .write_pkt(ScEntityEvent::new(
                                    0,
                                    EntityEvent::SetOpPermissionLevel4,
                                ))
                                .await?;
                            writer
                                .write_pkt(ScGameEvent::new(GameEvent::StartWaitingForChunks, 0.0))
                                .await?;
                            writer.write_pkt(ScSetCenterChunk::new(0, 0)).await?;
                            writer
                                .write_pkt(ScPlayerPosition::new(
                                    0, 0.5, 95.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0,
                                ))
                                .await?;

                            // brand
                            writer
                                .write_pkt(ScPluginMessage::new(
                                    "minecraft:brand".to_owned(),
                                    "black_hole".to_owned(),
                                ))
                                .await?;

                            // Player Info Update (tab list)
                            body = PacketBytes::new();
                            // 0x08 (list) | 0x01 (add player)
                            body.put_u8(0x09)?; // the bit

                            body.put_var_int(1)?; // collection len
                            body.put_uuid(ls.game_profile.uuid)?; // player uuid

                            body.put_string(ls.game_profile.username.clone())?; // player name
                            body.put_var_int(0)?; // count of properties

                            body.put_bool(true)?; // listed

                            writer.write_packet(0x44, body.to_vec()).await?;

                            // keep alive
                            writer.write_pkt(ScKeepAlive::new(1)).await?;

                            {
                                *state.write().await = ConnectionState::Play;
                            }
                        } else {
                            eprintln!("configuration received packet id {id:X}");
                        }
                    }
                    ConnectionState::Play => {
                        if id == CsMovePlayerRot::ID
                            || id == CsPlayerInput::ID
                            || id == CsPlayerCommand::ID
                            || id == CsChunkBatchReceived::ID
                            || id == CsSetCarriedItem::ID
                            || id == CsCustomPayload::ID
                            || id == CsAcceptTeleportation::ID
                            || id == CsPlayerLoaded::ID
                            || id == CsPlayerAction::ID
                            || id == CsSwing::ID
                            // only gets sent when F3 is open, practically useless
                            || id == CsPingRequest::ID
                        {
                            continue;
                        }

                        if id == CsClientTickEnd::ID {
                            if client_tick % 20 == 0 {
                                writer.write_pkt(ScKeepAlive::new(1)).await?;
                            }

                            client_tick += 1;
                            continue;
                        }

                        // client keep alive
                        if id == CsKeepAlive::ID {
                            continue;
                        }

                        // move player pos
                        if id == CsMovePlayerPos::ID {
                            let pkt = CsMovePlayerPos::decode(&mut data)?;

                            world
                                .send(WorldRequest::UpdatePlayerPosition {
                                    player,
                                    position: Vector3::new(pkt.x, pkt.y, pkt.z),
                                })
                                .await?;
                            continue;
                        }

                        // move player posrot
                        if id == CsMovePlayerPosRot::ID {
                            let pkt = CsMovePlayerPosRot::decode(&mut data)?;

                            world
                                .send(WorldRequest::UpdatePlayerPosition {
                                    player,
                                    position: Vector3::new(pkt.x, pkt.y, pkt.z),
                                })
                                .await?;
                            continue;
                        }

                        if id == CsPlayerAbilities::ID {
                            let pkt = CsPlayerAbilities::decode(&mut data)?;
                            let is_flying = pkt.flags == 0x02; // no other flags

                            world
                                .send(WorldRequest::UpdatePlayerFlying { player, is_flying })
                                .await?;
                            continue;
                        }

                        // change game mode
                        #[expect(clippy::cast_precision_loss)]
                        if id == CsChangeGameMode::ID {
                            let pkt = CsChangeGameMode::decode(&mut data)?;

                            writer
                                .write_pkt(ScGameEvent::new(
                                    GameEvent::ChangeGamemode,
                                    pkt.game_mode as i32 as f32,
                                ))
                                .await?;

                            // this is required for spectator mode noclip, for some reason
                            // Player Info Update (tab list)
                            body = PacketBytes::new();
                            // 0x04 (update game mode)
                            body.put_u8(0x04)?; // the bit

                            body.put_var_int(1)?; // collection len
                            body.put_uuid(ls.game_profile.uuid)?; // player uuid

                            body.put_var_int(pkt.game_mode as i32)?; // game mode

                            writer.write_packet(0x44, body.to_vec()).await?;

                            world
                                .send(WorldRequest::UpdatePlayerGameMode {
                                    player,
                                    game_mode: pkt.game_mode,
                                })
                                .await?;
                            continue;
                        }

                        // chat command
                        if id == CsChatCommand::ID {
                            let pkt = CsChatCommand::decode(&mut data)?;
                            let command = pkt.command;

                            if command.starts_with("fs ") {
                                let speed = command
                                    .split("fs ")
                                    .nth(1)
                                    .ok_or_else(|| anyhow::anyhow!("this should never happen"))?;
                                let fly_speed = speed.parse()?;
                                let game_mode = world.get_player_game_mode(player).await?;
                                let flying = world.get_player_flying(player).await?;

                                let mut flags = 0x00;

                                if game_mode == GameMode::Creative {
                                    flags |= 0x01; // invulnerable
                                    flags |= 0x04; // allow flying
                                    flags |= 0x08; // instant break
                                } else if game_mode == GameMode::Spectator {
                                    flags |= 0x02; // flying
                                    flags |= 0x04; // allow flying
                                }

                                if flying {
                                    flags |= 0x02; // flying
                                }

                                writer
                                    .write_pkt(ScPlayerAbilities::new(flags, fly_speed, 0.1))
                                    .await?;
                                continue;
                            }

                            if command == "surface" {
                                writer
                                    .write_pkt(ScPlayerPosition::new(
                                        0, 0.5, 72.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0,
                                    ))
                                    .await?;
                                continue;
                            }

                            if command == "stop" {
                                break;
                            }

                            if command.starts_with("tp ") {
                                let x = command
                                    .split("tp ")
                                    .nth(1)
                                    .ok_or_else(|| anyhow::anyhow!("this should never happen"))?
                                    .split(' ')
                                    .next()
                                    .ok_or_else(|| anyhow::anyhow!("bad x value"))?
                                    .trim();
                                let x = x.parse()?;
                                let y = command
                                    .split("tp ")
                                    .nth(1)
                                    .ok_or_else(|| anyhow::anyhow!("this should never happen"))?
                                    .split(' ')
                                    .nth(1)
                                    .ok_or_else(|| anyhow::anyhow!("bad y value"))?
                                    .trim();
                                let y = y.parse()?;
                                let z = command
                                    .split("tp ")
                                    .nth(1)
                                    .ok_or_else(|| anyhow::anyhow!("this should never happen"))?
                                    .split(' ')
                                    .nth(2)
                                    .ok_or_else(|| anyhow::anyhow!("bad z value"))?
                                    .trim();
                                let z = z.parse()?;
                                writer
                                    .write_pkt(ScPlayerPosition::new(
                                        0, x, y, z, 0.0, 0.0, 0.0, 0.0, 0.0, 0,
                                    ))
                                    .await?;
                                continue;
                            }

                            if command == "metaball" {
                                let mut position = world.get_player_position(player).await?;
                                position += Vector3::new(0.0, 15.0, 0.0);
                                let position = Vector3::new(
                                    position.x as i32,
                                    position.y as i32,
                                    position.z as i32,
                                );
                                world
                                    .send(WorldRequest::AddMetaball {
                                        position,
                                        perma: false,
                                    })
                                    .await?;
                                continue;
                            }

                            if command == "metaballp" {
                                let mut position = world.get_player_position(player).await?;
                                position += Vector3::new(0.0, 15.0, 0.0);
                                let position = Vector3::new(
                                    position.x as i32,
                                    position.y as i32,
                                    position.z as i32,
                                );
                                world
                                    .send(WorldRequest::AddMetaball {
                                        position,
                                        perma: true,
                                    })
                                    .await?;
                                continue;
                            }

                            // System Chat Message
                            body = PacketBytes::new();
                            body.put_u8(0x08)?; // TAG_String
                            body.put_u8(0x00)?; // TAG_END?
                            body.put_str("unknown command!")?; // text
                            body.put_u8(0x00)?; // overlay

                            writer.write_packet(0x77, body.to_vec()).await?;
                            continue;
                        }

                        println!("play received packet id {id:X}");
                    }
                    _ => unreachable!(),
                }
            }
            Err(e) => {
                println!("Connection closed: {e:?}");
                break;
            }
        }
    }

    world.send(WorldRequest::Stop).await?;
    handle.await?;
    ticker_handle.abort();

    Ok(())
}
