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
