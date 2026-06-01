use fastnbt::ByteArray;

// use uuid::Uuid;
use crate::{codecs::array::Array, proto::varint::VarInt, quickpkt};

quickpkt!(sc_keep_alive, 0x2B, rand => i64);

quickpkt!(sc_login, 0x30,
    entity_id => i32,
    is_hardcore => bool,
    dimensions => Array<String>,
    max_players => VarInt,
    view_distance => VarInt,
    simulation_distance => VarInt,
    reduced_debug_info => bool,
    respawn_screen => bool,
    limited_crafting => bool,
    dimension_type => VarInt,
    dimension => String,
    seed => i64,
    gamemode => u8,
    prev_gamemode => u8,
    is_debug => bool,
    is_flat => bool,
    // if this is present there are 2 more fields
    // but we dont have support for parsing that yet
    has_death_location => bool,
    portal_cooldown => VarInt,
    sea_level => VarInt,
    secure_chat => bool
);

quickpkt!(cs_move_player_pos_rot, 0x1E, x => f64, y => f64, z => f64, yaw => f32, pitch => f32, flags => u8);
quickpkt!(sc_plugin_message, 0x18, channel => String, data => ByteArray);

// pub struct ScoreboardPlayer {
//     uuid: Uuid,
//     actions: Array<ScoreboardPlayerAction>
// }

// quickpkt!(sc_player_info_update, 0x44, bitset => Array<i64>, players => Array<ScoreboardPlayer>);
