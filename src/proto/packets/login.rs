use uuid::Uuid;

use crate::{codecs::game_profile::GameProfile, proto::varint::VarInt, quickpkt};

quickpkt!(
    sc_set_compression,
    0x03, threshold => VarInt
);
quickpkt!(sc_login_success, 0x02, game_profile => GameProfile);
quickpkt!(
    cs_login_start,
    0x00,
    username => String,
    uuid => Uuid
);
