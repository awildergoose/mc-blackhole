use cgmath::Vector3;

use crate::{
    codecs::{base::MCDecode, game_profile::GameProfile},
    expect_packet,
    net::framing::FramedConn,
    proto::{
        packet_bytes::PacketBytes,
        packets::{
            Packet,
            config::{
                KnownPack, sc_select_known_packs::ScSelectKnownPacks,
                sc_update_enabled_features::ScUpdateEnabledFeatures,
            },
            handshake::cs_intention::CsIntention,
            login::{
                cs_login_start::CsLoginStart, sc_login_success::ScLoginSuccess,
                sc_set_compression::ScSetCompression,
            },
            play::{
                GameEvent, sc_game_event::ScGameEvent, sc_keep_alive::ScKeepAlive,
                sc_login::ScLogin, sc_plugin_message::ScPluginMessage,
            },
        },
        raw::{regs, tags},
    },
    world::{entity::PlayerEntity, level::Level},
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

    let sc = ScSetCompression { threshold: 256 };
    conn.write_pkt(sc.clone()).await?;
    conn.enable_compression(sc.threshold);

    let ls = ScLoginSuccess::new(GameProfile::new(login.uuid, login.username.clone()));
    conn.write_pkt(ls.clone()).await?;

    let mut client_tick = 0;
    let mut body;
    let mut level = Level::new(4);
    let player = level.add_entity(PlayerEntity::new(login.username));

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
                            conn.write_packet(0x07, regs::SECTION0).await?;
                            conn.write_packet(0x07, regs::SECTION1).await?;
                            conn.write_packet(0x07, regs::SECTION2).await?;
                            conn.write_packet(0x07, regs::SECTION3).await?;
                            conn.write_packet(0x07, regs::SECTION4).await?;
                            conn.write_packet(0x07, regs::SECTION5).await?;
                            conn.write_packet(0x07, regs::SECTION6).await?;
                            conn.write_packet(0x07, regs::SECTION7).await?;
                            conn.write_packet(0x07, regs::SECTION8).await?;
                            conn.write_packet(0x07, regs::SECTION9).await?;
                            conn.write_packet(0x07, regs::SECTION10).await?;
                            conn.write_packet(0x07, regs::SECTION11).await?;
                            conn.write_packet(0x07, regs::SECTION12).await?;
                            conn.write_packet(0x07, regs::SECTION13).await?;
                            conn.write_packet(0x07, regs::SECTION14).await?;
                            conn.write_packet(0x07, regs::SECTION15).await?;
                            conn.write_packet(0x07, regs::SECTION16).await?;
                            conn.write_packet(0x07, regs::SECTION17).await?;
                            conn.write_packet(0x07, regs::SECTION18).await?;
                            conn.write_packet(0x07, regs::SECTION19).await?;
                            conn.write_packet(0x07, regs::SECTION20).await?;

                            conn.write_packet(0x0D, tags::TAGS).await?;

                            // finish configuration
                            body = PacketBytes::new();
                            conn.write_packet(0x03, &body).await?;
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
                        if id == 0x1D {
                            let x = data.get_f64()?;
                            let y = data.get_f64()?;
                            let z = data.get_f64()?;
                            let _flags = data.get_u8()?;
                            level
                                .update_player_position(player, conn, Vector3::new(x, y, z))
                                .await?;

                            continue;
                        }

                        // move player posrot
                        if id == 0x1E {
                            let x = data.get_f64()?;
                            let y = data.get_f64()?;
                            let z = data.get_f64()?;
                            let _y_rot = data.get_f32()?;
                            let _x_rot = data.get_f32()?;
                            let _flags = data.get_u8()?;

                            level
                                .update_player_position(player, conn, Vector3::new(x, y, z))
                                .await?;
                            continue;
                        }

                        // move player rot, player input, player command, chunk batch received
                        if id == 0x1F || id == 0x2A || id == 0x29 || id == 0xA {
                            continue;
                        }

                        // ping request, only gets sent when F3 is open
                        // practically useless for our use case
                        if id == 0x25 {
                            continue;
                        }

                        // player command
                        if id == 0x6 {
                            body = PacketBytes::new();

                            // TODO: use an existing (s)nbt library
                            // manual SNBT!
                            body.put_u8(0x0A)?;

                            body.put_u8(0x08)?; // TAG_String
                            body.put_u16("text".len() as u16)?;
                            body.extend_from_slice(b"text");

                            body.put_u8(0x00)?; // TAG_End

                            body.put_u8(0x08)?; // TAG_String
                            body.put_u16("hello world".len() as u16)?;
                            body.extend_from_slice(b"hello world");

                            body.put_var_int(1)?;

                            // manual SNBT!
                            body.put_u8(0x0A)?;

                            body.put_u8(0x08)?; // TAG_String
                            body.put_u16("text".len() as u16)?;
                            body.extend_from_slice(b"text");

                            body.put_u8(0x00)?; // TAG_End

                            body.put_u8(0x08)?; // TAG_String
                            body.put_u16("hello world".len() as u16)?;
                            body.extend_from_slice(b"hello world");

                            body.put_u8(0)?;

                            conn.write_packet(0x21, &body).await?;

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
                                view_distance: level.view_distance,
                                simulation_distance: level.view_distance,
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

                            println!("sent login");

                            // game event
                            conn.write_pkt(ScGameEvent::new(GameEvent::StartWaitingForChunks, 0.0))
                                .await?;

                            // chunk

                            // set chunk center
                            body = PacketBytes::new();
                            body.put_var_int(0)?;
                            body.put_var_int(0)?;
                            conn.write_packet(0x5C, &body).await?;

                            // sync pos
                            body = PacketBytes::new();
                            body.put_var_int(0)?;

                            body.put_f64(0.5)?;
                            body.put_f64(72.0)?;
                            body.put_f64(0.5)?;

                            body.put_f64(0.0)?;
                            body.put_f64(0.0)?;
                            body.put_f64(0.0)?;

                            body.put_f32(0.0)?;
                            body.put_f32(0.0)?;

                            body.put_u32(0)?;
                            conn.write_packet(0x46, &body).await?;

                            println!("sent player pos sync");

                            // brand
                            conn.write_pkt(ScPluginMessage::new(
                                "minecraft:brand".to_owned(),
                                "black_hole".to_owned(),
                            ))
                            .await?;

                            // tab list
                            body = PacketBytes::new();
                            // 0x08 (list) | 0x01 (add player)
                            body.put_u8(0x09)?; // the bit

                            body.put_var_int(1)?; // collection len
                            body.put_uuid(ls.game_profile.uuid)?; // player uuid

                            body.put_string(ls.game_profile.username.clone())?; // player name
                            body.put_var_int(0)?; // count of properties

                            body.put_bool(true)?; // listed

                            conn.write_packet(0x44, &body).await?;

                            // TODO: send set_default_spawn_position

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
