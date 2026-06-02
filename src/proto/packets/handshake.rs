use crate::{proto::varint::VarInt, quickpkt};

quickpkt!(
    cs_intention, 0x00,
    protocol_version => VarInt,
    server_address => String,
    server_port => u16,
    intent => VarInt
);
