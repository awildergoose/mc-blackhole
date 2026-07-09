use cgmath::Vector3;
use tokio::sync::mpsc;

use crate::{
    codecs::{base::MCDecode, game_profile::GameProfile},
    expect_packet,
    net::framing::FramedConn,
    proto::{
        packet_bytes::PacketBytes,
        packets::{
            Packet,
            config::{
                KnownPack, sc_finish_configuration::ScFinishConfiguration,
                sc_registries::ScRegistries, sc_select_known_packs::ScSelectKnownPacks,
                sc_tags::ScTags, sc_update_enabled_features::ScUpdateEnabledFeatures,
            },
            handshake::cs_intention::CsIntention,
            login::{
                cs_login_start::CsLoginStart, sc_login_success::ScLoginSuccess,
                sc_set_compression::ScSetCompression,
            },
            play::{
                EntityEvent, GameEvent, cs_change_game_mode::CsChangeGameMode,
                cs_chat_command::CsChatCommand, cs_move_player_pos::CsMovePlayerPos,
                cs_move_player_pos_rot::CsMovePlayerPosRot,
                sc_chunk_batch_finished::ScChunkBatchFinished,
                sc_chunk_batch_start::ScChunkBatchStart, sc_entity_event::ScEntityEvent,
                sc_game_event::ScGameEvent, sc_keep_alive::ScKeepAlive,
                sc_level_chunk_with_light::ScLevelChunkWithLight, sc_login::ScLogin,
                sc_player_abilities::ScPlayerAbilities, sc_player_position::ScPlayerPosition,
                sc_plugin_message::ScPluginMessage, sc_set_center_chunk::ScSetCenterChunk,
            },
        },
        raw::{regs, tags},
    },
    world::{
        entity::PlayerEntity,
        level::{Level, UPPPacket},
        palette::PaletteBlockKind,
        worker::WorldWorker,
    },
};

pub enum ConnectionState {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

#[allow(clippy::too_many_lines)]
pub async fn handle_connection(conn: &mut FramedConn) -> anyhow::Result<()> {
    let mut state: ConnectionState; // Handshaking
    let intention = expect_packet!(conn, CsIntention)?;
    if intention.intent != 2 {
        anyhow::bail!("not a login intention");
    }
    let login = expect_packet!(conn, CsLoginStart)?;

    state = ConnectionState::Login;

    let sc = ScSetCompression::new(256);
    conn.write_pkt(sc.clone()).await?;
    conn.enable_compression(sc.threshold);

    let ls = ScLoginSuccess::new(GameProfile::new(login.uuid, login.username));
    conn.write_pkt(ls.clone()).await?;

    let mut client_tick = 0;
    let mut body;
    let mut level = Level::new(16);
    let player = level.add_entity(PlayerEntity::new());

    level.add_metaball(Vector3::new(0, 0, 0), 8.0, 2.0, PaletteBlockKind::OakPlanks);

    level.set_block(0, 20, 0, PaletteBlockKind::Grass);

    let (worker, world) = WorldWorker::new(level);

    tokio::spawn(async move {
        if let Err(e) = worker.run().await {
            eprintln!("world worker panicked: {e:?}");
        }
    });

    loop {
        match conn.read_packet().await {
            Ok((id, mut data)) => {
                match state {
                    ConnectionState::Login => {
                        if id == 0x03 {
                            // update enabled features
                            conn.write_pkt(ScUpdateEnabledFeatures {
                                features: vec!["minecraft:vanilla".to_owned()],
                            })
                            .await?;

                            // send known packs
                            conn.write_pkt(ScSelectKnownPacks {
                                features: vec![KnownPack::new(
                                    "minecraft".to_owned(),
                                    "core".to_owned(),
                                    "1.21.10".to_owned(),
                                )],
                            })
                            .await?;

                            state = ConnectionState::Configuration;
                        } else {
                            eprintln!("login received packet id {id:X}");
                        }
                    }
                    ConnectionState::Configuration => {
                        if id == 0x00 {
                            // registries
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION0)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION1)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION2)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION3)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION4)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION5)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION6)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION7)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION8)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION9)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION10)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION11)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION12)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION13)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION14)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION15)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION16)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION17)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION18)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION19)))
                                .await?;
                            conn.write_pkt(ScRegistries::new(Vec::from(regs::SECTION20)))
                                .await?;
                            conn.write_pkt(ScTags::new(Vec::from(tags::TAGS))).await?;
                            conn.write_pkt(ScFinishConfiguration::new()).await?;
                            state = ConnectionState::Play;
                        } else {
                            eprintln!("configuration received packet id {id:X}");
                        }
                    }
                    ConnectionState::Play => {
                        // client tick end
                        if id == 0x0C {
                            if client_tick % 20 == 0 {
                                conn.write_pkt(ScKeepAlive::new(1)).await?;
                            }

                            client_tick += 1;
                            continue;
                        }

                        // client keep alive
                        if id == 0x1B {
                            continue;
                        }

                        // move player pos
                        if id == CsMovePlayerPos::ID {
                            let pkt = CsMovePlayerPos::decode(&mut data)?;
                            let (sender, mut rcv) = mpsc::channel(16);

                            world
                                .update_player_position(
                                    player,
                                    Vector3::new(pkt.x, pkt.y, pkt.z),
                                    sender,
                                )
                                .await?;

                            while let Some(pk) = rcv.recv().await {
                                match pk {
                                    UPPPacket::Begin(x, z) => {
                                        conn.write_pkt(ScSetCenterChunk::new(x, z)).await?;
                                        conn.write_pkt(ScChunkBatchStart::new()).await?;
                                    }
                                    UPPPacket::Chunk(chunk) => {
                                        conn.write_pkt(ScLevelChunkWithLight::new(chunk.encode()?))
                                            .await?;
                                    }
                                    UPPPacket::Finish(count) => {
                                        conn.write_pkt(ScChunkBatchFinished::new(count)).await?;
                                    }
                                }
                            }

                            continue;
                        }

                        // move player posrot
                        if id == CsMovePlayerPosRot::ID {
                            let pkt = CsMovePlayerPosRot::decode(&mut data)?;
                            let (sender, mut rcv) = mpsc::channel(16);

                            world
                                .update_player_position(
                                    player,
                                    Vector3::new(pkt.x, pkt.y, pkt.z),
                                    sender,
                                )
                                .await?;

                            while let Some(pk) = rcv.recv().await {
                                match pk {
                                    UPPPacket::Begin(x, z) => {
                                        conn.write_pkt(ScSetCenterChunk::new(x, z)).await?;
                                        conn.write_pkt(ScChunkBatchStart::new()).await?;
                                    }
                                    UPPPacket::Chunk(chunk) => {
                                        conn.write_pkt(ScLevelChunkWithLight::new(chunk.encode()?))
                                            .await?;
                                    }
                                    UPPPacket::Finish(count) => {
                                        conn.write_pkt(ScChunkBatchFinished::new(count)).await?;
                                    }
                                }
                            }

                            continue;
                        }

                        // move player rot, player input, player command, chunk batch received, set carried item
                        if id == 0x1F || id == 0x2A || id == 0x29 || id == 0xA || id == 0x34 {
                            continue;
                        }

                        // ping request, only gets sent when F3 is open
                        // practically useless for our use case
                        if id == 0x25 {
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

                                conn.write_pkt(ScPlayerAbilities::new(0x04, fly_speed, 0.1))
                                    .await?;
                                continue;
                            }

                            if command == "surface" {
                                conn.write_pkt(ScPlayerPosition::new(
                                    0, 0.5, 72.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0,
                                ))
                                .await?;
                                continue;
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
                                conn.write_pkt(ScPlayerPosition::new(
                                    0, x, y, z, 0.0, 0.0, 0.0, 0.0, 0.0, 0,
                                ))
                                .await?;
                                continue;
                            }

                            // System Chat Message
                            body = PacketBytes::new();
                            body.put_u8(0x08)?; // TAG_String
                            body.put_u8(0x00)?; // TAG_END?
                            body.put_str("unknown command!")?; // text
                            body.put_u8(0x00)?; // overlay

                            conn.write_packet(0x77, &body).await?;
                            continue;
                        }

                        // change game mode
                        if id == CsChangeGameMode::ID {
                            let pkt = CsChangeGameMode::decode(&mut data)?;

                            #[allow(clippy::cast_precision_loss)]
                            conn.write_pkt(ScGameEvent::new(
                                GameEvent::ChangeGamemode,
                                pkt.game_mode as f32,
                            ))
                            .await?;
                            continue;
                        }

                        println!("play received packet id {id:X}");

                        if id == 0x03 {
                            // we're now in play, lets send Login
                            conn.write_pkt(ScLogin {
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
                                gamemode: 1,
                                prev_gamemode: 0xFF,
                                is_debug: false,
                                is_flat: false,
                                has_death_location: false,
                                portal_cooldown: 0,
                                sea_level: 63,
                                secure_chat: false,
                            })
                            .await?;

                            conn.write_pkt(ScEntityEvent::new(
                                0,
                                EntityEvent::SetOpPermissionLevel4,
                            ))
                            .await?;
                            conn.write_pkt(ScGameEvent::new(GameEvent::StartWaitingForChunks, 0.0))
                                .await?;
                            conn.write_pkt(ScSetCenterChunk::new(0, 0)).await?;
                            conn.write_pkt(ScPlayerPosition::new(
                                0, 0.5, 95.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0,
                            ))
                            .await?;

                            // brand
                            conn.write_pkt(ScPluginMessage::new(
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

                            conn.write_packet(0x44, &body).await?;

                            // keep alive
                            conn.write_pkt(ScKeepAlive { rand: 1 }).await?;
                        }
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

    Ok(())
}
