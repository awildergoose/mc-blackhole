use crate::{
    codecs::{array::Array, enums::Enum, position::Position, string8::String8},
    create_enum,
    proto::{packet_bytes::PacketBytes, varint::VarInt},
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

quickpkt!(cs_move_player_pos, 0x1D, x => f64, y => f64, z => f64, flags => u8);
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

create_enum!(EntityEvent, u8,
    // limited set
    SetOpPermissionLevel0 => 24,
    SetOpPermissionLevel1 => 25,
    SetOpPermissionLevel2 => 26,
    SetOpPermissionLevel3 => 27,
    SetOpPermissionLevel4 => 28
);
quickpkt!(sc_entity_event, 0x22, entity_id => i32, status => Enum<EntityEvent>);

quickpkt!(sc_player_abilities, 0x3E, flags => u8, flying_speed => f32, fov_modifier => f32);
quickpkt!(cs_change_game_mode, 0x04, game_mode => VarInt);
quickpkt!(cs_chat_command, 0x06, command => String);
quickpkt!(sc_set_center_chunk, 0x5C, x => VarInt, z => VarInt);
quickpkt!(sc_chunk_batch_start, 0x0C);
quickpkt!(sc_chunk_batch_finished, 0x0B, batch_size => VarInt);

// NOTE: The actual packet structure is manually constructed in Chunk::encode!
// We don't use RemainingArray here because it would be incredibly slow.
quickpkt!(sc_level_chunk_with_light, 0x2C, bytes => PacketBytes);
quickpkt!(sc_block_update, 0x08, location => Position, global_block_id => VarInt);
