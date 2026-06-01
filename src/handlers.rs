use bytes::{BufMut, BytesMut};
use uuid::Uuid;

use crate::{
    codecs::base::{MCDecode, MCEncode},
    net::framing::FramedConn,
    proto::{
        game_profile::GameProfile,
        packet_bytes::PacketBytes,
        packets::{
            Packet,
            login::{
                login_start::LoginStart, login_success::LoginSuccess,
                set_compression::SetCompression,
            },
            play::sc_login::ScLogin,
        },
        rawchunktest::CHUNK_TEST,
        regs::{self},
        tags::TAGS,
    },
};

#[allow(clippy::too_many_lines)]
pub async fn handle_connection(conn: &mut FramedConn) -> anyhow::Result<()> {
    let (id, mut payload) = conn.read_packet().await?;
    if id != 0x00 {
        anyhow::bail!("not a handshake");
    }
    let _proto = payload.get_var_int()?;
    let _server_address = payload.get_string()?;
    if payload.len() < 2 {
        anyhow::bail!("bad handshake");
    }
    let _ = payload.split_to(2);
    let next_state = payload.get_var_int()?;
    if next_state != 2 {
        anyhow::bail!("next_state != 2");
    }

    let (id, mut payload) = conn.read_packet().await?;
    if id != LoginStart::ID {
        anyhow::bail!("expected LoginStart");
    }
    let login = LoginStart::decode(&mut payload)?;
    println!("LoginStart username={}", login.username);

    let sc = SetCompression { threshold: 256 };
    let mut sc_body = PacketBytes::new();
    sc.encode(&mut sc_body)?;
    conn.write_packet(SetCompression::ID, &sc_body).await?;
    conn.enable_compression(sc.threshold);
    println!("Sent SetCompression");

    let ls = LoginSuccess::new(GameProfile {
        username: login.username,
        uuid: Uuid::new_v4(),
    });
    let mut body = PacketBytes::new();
    ls.encode(&mut body)?;
    conn.write_packet(LoginSuccess::ID, &body).await?;
    println!("Sent LoginSuccess");

    let mut state = 0; // 0 = configure, 1 = play
    let mut client_tick = 0;

    loop {
        match conn.read_packet().await {
            Ok((id, _data)) => {
                match state {
                    0 => {
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

                            conn.write_packet(0x0D, TAGS).await?;

                            // finish configuration
                            body = PacketBytes::new();
                            conn.write_packet(0x03, &body).await?;
                            state = 1;
                            continue;
                        }
                        if id == 0x03 {
                            // we're in configuration right now, become play
                            println!("login ack");

                            // update enabled features
                            body = PacketBytes::new();
                            body.extend_from_slice(&[
                                0x01, 0x11, 0x6D, 0x69, 0x6E, 0x65, 0x63, 0x72, 0x61, 0x66, 0x74,
                                0x3A, 0x76, 0x61, 0x6E, 0x69, 0x6C, 0x6C, 0x61,
                            ]);
                            conn.write_packet(0x0C, &body).await?;

                            let known_packs = [
                                0x01, 0x09, 0x6D, 0x69, 0x6E, 0x65, 0x63, 0x72, 0x61, 0x66, 0x74,
                                0x04, 0x63, 0x6F, 0x72, 0x65, 0x07, 0x31, 0x2E, 0x32, 0x31, 0x2E,
                                0x31, 0x30,
                            ];
                            body = PacketBytes::new();
                            body.extend_from_slice(&known_packs);
                            conn.write_packet(0x0E, &body).await?;
                        } else {
                            println!("Post-login received packet id {id:x}");
                        }
                    }
                    1 => {
                        // client tick end
                        if id == 0x0C {
                            if client_tick % 20 == 0 {
                                body = PacketBytes::new();
                                body.put_i64(1)?;
                                conn.write_packet(0x2B, &body).await?;
                            }

                            client_tick += 1;
                            continue;
                        }

                        // client keep alive
                        if id == 0x1B {
                            continue;
                        }

                        println!("Play packet received packet id {id:X}");

                        if id == 0x03 {
                            // we're now in play, lets send Login
                            conn.write_packet(
                                ScLogin::ID,
                                &ScLogin {
                                    entity_id: 0,
                                    is_hardcore: false,
                                    dimensions: vec!["overworld".to_owned()],
                                    max_players: 1,
                                    view_distance: 1,
                                    simulation_distance: 1,
                                    reduced_debug_info: false,
                                    respawn_screen: true,
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
                                }
                                .encoded()?,
                            )
                            .await?;

                            println!("sent login");

                            // game event
                            body = PacketBytes::new();
                            body.put_u8(13)?;
                            body.put_f32(0.0)?;
                            conn.write_packet(0x26, &body).await?;

                            // chunk
                            conn.write_packet(0x0C, &[]).await?;
                            let r = 4;
                            for x in -r..r {
                                for z in -r..r {
                                    let mut chkbody = BytesMut::new();
                                    chkbody.put_i32(x);
                                    chkbody.put_i32(z);
                                    chkbody.extend_from_slice(CHUNK_TEST);
                                    conn.write_packet(0x2C, &chkbody).await?;
                                }
                            }
                            conn.write_packet(0x0B, &[0x01]).await?;

                            println!("sent chunk");

                            // player spawn
                            conn.write_packet(0x00, &[]).await?;
                            body = PacketBytes::new();
                            body.put_var_int(0)?; // ent id
                            body.put_uuid(ls.game_profile.uuid)?; // uuid
                            body.put_var_int(10)?; // ent type
                            body.put_f64(0.0)?; // x
                            body.put_f64(0.0)?; // y
                            body.put_f64(0.0)?; // z
                            body.put_u8(0)?; // movement lpvec3 https://minecraft.wiki/w/Java_Edition_protocol/Data_types#LpVec3
                            body.put_u8(0)?; // pitch
                            body.put_u8(0)?; // yaw
                            body.put_u8(0)?; // head yaw
                            body.put_var_int(0)?; // data
                            conn.write_packet(0x01, &body).await?;

                            // ent data

                            conn.write_packet(
                                0x61,
                                &[
                                    0x00, // ent id
                                    0x09, 0x03, 0x40, 0xC0, 0x00, 0x00, 0x10, 0x00, 0x01, 0xFF,
                                ],
                            )
                            .await?;

                            conn.write_packet(0x00, &[]).await?;

                            println!("sent player info");

                            // sync pos
                            body = PacketBytes::new();
                            body.put_var_int(0)?;

                            body.put_f64(0.0)?;
                            body.put_f64(64.0)?;
                            body.put_f64(0.0)?;

                            body.put_f64(0.0)?;
                            body.put_f64(0.0)?;
                            body.put_f64(0.0)?;

                            body.put_f32(0.0)?;
                            body.put_f32(0.0)?;

                            body.put_u32(0)?;
                            conn.write_packet(0x46, &body).await?;

                            // keep alive
                            body = PacketBytes::new();
                            body.put_i64(1)?;
                            conn.write_packet(0x2B, &body).await?;

                            println!("sent player pos sync");
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
