use crate::codecs::base::{MCDecode, MCEncode};

pub type Array<T>
    = Vec<T>
where
    T: MCEncode + MCDecode;
