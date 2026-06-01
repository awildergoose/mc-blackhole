use fastnbt::ByteArray;

use crate::{
    proto::{game_profile::GameProfile, varint::VarInt},
    quickpkt,
};

quickpkt!(
    set_compression,
    0x03, threshold => VarInt
);
quickpkt!(login_success, 0x02, game_profile => GameProfile);
quickpkt!(
    login_start,
    0x00,
    username => String
);
quickpkt!(sc_plugin_message, 0x1B, channel => String, data => ByteArray);
