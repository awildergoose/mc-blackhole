use crate::{
    codecs::{
        array::{Array, RemainingArray},
        enums::Enum,
        position::Position,
        string8::String8,
    },
    create_enum, create_enum_varint,
    proto::{packet_bytes::PacketBytes, varint::VarInt},
    quickpkt,
};

create_enum_varint!(PlayerActionId,
    LeaveBed => 0,
    StartSprinting => 1,
    StopSprinting => 2,
    StartJumpWithHorse => 3,
    StopJumpWithHorse => 4,
    OpenVehicleInventory => 5,
    StartFlyingWithElytra => 6
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
create_enum!(EntityEvent, u8,
    // limited set
    SetOpPermissionLevel0 => 24,
    SetOpPermissionLevel1 => 25,
    SetOpPermissionLevel2 => 26,
    SetOpPermissionLevel3 => 27,
    SetOpPermissionLevel4 => 28
);
create_enum_varint!(PlayerActionStatus,
    StartedDigging => 0,
    CancelledDigging => 1,
    FinishedDigging => 2,
    DropItemStack => 3,
    DropItem => 4,
    ShootArrow => 5,
    SwapItemInHand => 6
);

// N == negative, P == positive
create_enum!(PlayerActionFace, u8,
    NY => 0,
    PY => 1,
    NZ => 2,
    PZ => 3,
    NX => 4,
    PX => 5
);

create_enum_varint!(GameMode,
    Survival => 0,
    Creative => 1,
    Adventure => 2,
    Spectator => 3
);

// Really dumb, but just incase :')
create_enum!(ByteGameMode, u8,
    Survival => 0,
    Creative => 1,
    Adventure => 2,
    Spectator => 3
);

create_enum_varint!(PlayerHand,
    Main => 0,
    Off => 1
);

quickpkt!(sc_bundle_delimiter, 0x00);
quickpkt!(sc_keep_alive, 0x2B, rand => i64);
quickpkt!(cs_keep_alive, 0x1B);
quickpkt!(cs_client_tick_end, 0x0C);
quickpkt!(cs_player_input, 0x2A, flags => u8);
quickpkt!(cs_player_command, 0x29, entity_id => VarInt, action_id => Enum<PlayerActionId>, jump_boost => VarInt);
quickpkt!(cs_chunk_batch_received, 0x0A, chunks_per_tick => f32);
quickpkt!(cs_set_carried_item, 0x34, slot => i16);
quickpkt!(cs_ping_request, 0x25, payload => i64);

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
    gamemode => Enum<ByteGameMode>,
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
quickpkt!(cs_move_player_rot, 0x1F, yaw => f32, pitch => f32, flags => u8);
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

quickpkt!(sc_game_event, 0x26, event => Enum<GameEvent>, value => f32);
quickpkt!(sc_entity_event, 0x22, entity_id => i32, status => Enum<EntityEvent>);
quickpkt!(sc_player_abilities, 0x3E, flags => u8, flying_speed => f32, fov_modifier => f32);
quickpkt!(cs_player_abilities, 0x27, flags => u8);

quickpkt!(cs_player_action, 0x28, status => Enum<PlayerActionStatus>, location => Position, face => Enum<PlayerActionFace>, sequence => VarInt);
quickpkt!(cs_change_game_mode, 0x04, game_mode => Enum<GameMode>);
quickpkt!(cs_chat_command, 0x06, command => String);

quickpkt!(sc_set_center_chunk, 0x5C, x => VarInt, z => VarInt);
quickpkt!(sc_chunk_batch_start, 0x0C);
quickpkt!(sc_chunk_batch_finished, 0x0B, batch_size => VarInt);
// NOTE: The actual packet structure is manually constructed in Chunk::encode!
// We don't use RemainingArray here because it would be incredibly slow.
quickpkt!(sc_level_chunk_with_light, 0x2C, bytes => PacketBytes);
quickpkt!(sc_forget_level_chunk, 0x25, z => i32, x => i32);
quickpkt!(sc_block_update, 0x08, location => Position, global_block_id => VarInt);
quickpkt!(sc_block_changed_ack, 0x04, sequence => VarInt);
quickpkt!(cs_use_item_on, 0x3F,
    hand => Enum<PlayerHand>,
    location => Position,
    face => Enum<PlayerActionFace>,
    cursor_pos_x => f32,
    cursor_pos_y => f32,
    cursor_pos_z => f32,
    inside_block => bool,
    world_border_hit => bool,
    sequence => VarInt
);

quickpkt!(cs_custom_payload, 0x15, channel => String, data => RemainingArray<u8>);
quickpkt!(cs_accept_teleportation, 0x00, id => u8);
quickpkt!(cs_player_loaded, 0x2B);
quickpkt!(cs_swing, 0x3C, hand => Enum<PlayerHand>);
