use crate::{
    codecs::{array::Array, enums::Enum, string8::String8},
    create_enum,
    proto::varint::VarInt,
    quickpkt,
};

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

// Technically we can do more than strings here but it's fine for
// our use case for now of minecraft:brand
quickpkt!(sc_plugin_message, 0x18, channel => String, data => String8);
quickpkt!(sc_player_position, 0x46,
    teleport_id => VarInt,
    x => f64, y => f64, z => f64,
    vx => f64, vy => f64, vz => f64,
    yaw => f32, pitch => f32,
    flags => u32
);

create_enum!(GameEvent, u8,
    NoRespawnBlockAvailable => 0,
    BeginRaining => 1,
    EndRaining => 2,
    ChangeGamemode => 3,
    WinGame => 4,
    DemoEvent => 5,
    ArrowHitPlayer => 6,
    RainLevelChange => 7,
    ThunderLevelChange => 8,
    PlayPufferfishStingSound => 9,
    PlayElderGuardianAppearance => 10,
    EnableRespawnScreen => 11,
    LimitedCrafting => 12,
    StartWaitingForChunks => 13
);
quickpkt!(sc_game_event, 0x26, event => Enum<GameEvent>, value => f32);
