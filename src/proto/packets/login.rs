use crate::{
    proto::{game_profile::GameProfile, varint::VarInt},
    quickpkt,
};

quickpkt!(
    sc_set_compression,
    0x03, threshold => VarInt
);
quickpkt!(sc_login_success, 0x02, game_profile => GameProfile);
quickpkt!(
    sc_login_start,
    0x00,
    username => String
);
