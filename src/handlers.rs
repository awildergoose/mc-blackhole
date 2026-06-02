use bytes::{BufMut, BytesMut};
use fastnbt::ByteArray;

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
        raw::{rawchunktest, regs, tags},
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

    let sc = ScSetCompression { threshold: 256 };
    conn.write_pkt(sc.clone()).await?;
    conn.enable_compression(sc.threshold);

    let ls = ScLoginSuccess::new(GameProfile::new(login.uuid, login.username));
    conn.write_pkt(ls.clone()).await?;

    let mut client_tick = 0;
    let mut body;

    loop {
        match conn.read_packet().await {
            Ok((id, _data)) => {
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

                        println!("play received packet id {id:X}");

                        if id == 0x03 {
                            // we're now in play, lets send Login
                            conn.write_pkt(ScLogin {
                                entity_id: 0,
                                is_hardcore: false,
                                dimensions: vec!["overworld".to_owned()],
                                max_players: 1,
                                view_distance: 32,
                                simulation_distance: 32,
                                reduced_debug_info: false,
                                respawn_screen: true,
                                limited_crafting: false,
                                dimension_type: 0,
                                dimension: "overworld".to_owned(),
                                seed: 0,
                                gamemode: 3,
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

                            // chunks begin
                            conn.write_packet(0x0C, &[]).await?;

                            println!("sending chunks");
                            let r = 32;
                            for x in -r..r {
                                for z in -r..r {
                                    let mut chkbody = BytesMut::new();
                                    chkbody.put_i32(x);
                                    chkbody.put_i32(z);
                                    chkbody.extend_from_slice(rawchunktest::CHUNK_TEST);
                                    conn.write_packet(0x2C, &chkbody).await?;
                                }
                            }
                            println!("sent chunks");

                            // chunks end
                            conn.write_packet(0x0B, &[0x01]).await?;

                            // sync pos
                            body = PacketBytes::new();
                            body.put_var_int(0)?;

                            body.put_f64(0.0)?;
                            body.put_f64(72.0)?;
                            body.put_f64(0.0)?;

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
                                ByteArray::new(
                                    Vec::from(b"\x0Ablack_hole")
                                        .iter()
                                        .map(|c| *c as i8)
                                        .collect(),
                                ),
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
