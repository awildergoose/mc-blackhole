use crate::codecs::base::{MCDecode, MCEncode};

pub type Enum<T>
    = T
where
    T: MCEncode + MCDecode;
