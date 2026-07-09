use crate::codecs::base::{MCDecode, MCEncode};

pub type Array<T>
    = Vec<T>
where
    T: MCEncode + MCDecode;

/// This treats the rest of the bytes as a single array.
/// This is meant to only be used as a band-aid.
pub type RemainingArray<T>
    = Vec<T>
where
    T: MCEncode + MCDecode;
