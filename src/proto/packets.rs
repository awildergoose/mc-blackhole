use crate::{
    codecs::base::{MCDecode, MCEncode},
    proto::packet_bytes::PacketBytes,
};

pub mod config;
pub mod handshake;
pub mod login;
pub mod play;

pub trait Packet: MCEncode + MCDecode {
    const ID: i32;

    fn encoded(&self) -> anyhow::Result<PacketBytes> {
        let mut dst = PacketBytes::new();
        self.encode(&mut dst)?;
        Ok(dst)
    }
}

#[macro_export]
macro_rules! create_enum {
    ($name:tt, $ftype:ty, $($tname:ty => $tvalue:tt),*) => {
        paste::paste! {
            #[derive(Copy, Clone, Debug, PartialEq, Eq)]
            #[repr($ftype)]
            pub enum $name {
                $(
                    $tname = $tvalue,
                )*
            }

            impl $crate::codecs::base::MCEncode for $name {
                fn encode(&self, dst: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<()> {
                    generics_macro::put_ident!(dst, $ftype, match self {
                        $(
                            Self::$tname => $tvalue,
                        )*
                    });
                    Ok(())
                }
            }

            impl $crate::codecs::base::MCDecode for $name {
                fn decode(src: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<Self> {
                    let v = generics_macro::get_ident!(src, $ftype);
                    Ok(match v {
                        $(
                            $tvalue => Self::$tname,
                        )*
                        _ => anyhow::bail!("invalid value for enum {}, got {v}", stringify!($name)),
                    })
                }
            }
        }
    };
}

#[macro_export]
macro_rules! create_enum_varint {
    ($name:tt, $($tname:ty => $tvalue:tt),*) => {
        paste::paste! {
            #[derive(Copy, Clone, Debug, PartialEq, Eq)]
            #[repr(i32)]
            pub enum $name {
                $(
                    $tname = $tvalue,
                )*
            }

            impl $crate::codecs::base::MCEncode for $name {
                fn encode(&self, dst: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<()> {
                    generics_macro::put_ident!(dst, var_int, match self {
                        $(
                            Self::$tname => $tvalue,
                        )*
                    });
                    Ok(())
                }
            }

            impl $crate::codecs::base::MCDecode for $name {
                fn decode(src: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<Self> {
                    let v = generics_macro::get_ident!(src, var_int);
                    Ok(match v {
                        $(
                            $tvalue => Self::$tname,
                        )*
                        _ => anyhow::bail!("invalid value for enum {}, got {v}", stringify!($name)),
                    })
                }
            }
        }
    };
}

#[macro_export]
macro_rules! create_codec {
    ($name:tt) => {
        #[derive(Clone, Debug)]
        pub struct $name;

        impl $name {
            #[must_use]
            pub const fn new() -> Self {
                Self
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self
            }
        }

        impl $crate::codecs::base::MCEncode for $name {
            fn encode(&self, _dst: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<()> {
                Ok(())
            }
        }

        impl $crate::codecs::base::MCDecode for $name {
            fn decode(_src: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<Self> {
                Ok(Self)
            }
        }
    };
    ($name:tt, $($fname:tt => $ftype:ty),*) => {
        #[derive(Clone, Debug)]
        pub struct $name {
            $(pub $fname: $ftype),*,
        }

        impl $name {
            #[allow(clippy::too_many_arguments)]
            #[must_use]
            pub const fn new($($fname: $ftype),*) -> Self {
                Self {
                    $(
                        $fname,
                    )*
                }
            }
        }

        impl $crate::codecs::base::MCEncode for $name {
            fn encode(&self, dst: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<()> {
                $(
                    generics_macro::put_ident!(dst, $ftype, self.$fname.clone());
                )*
                Ok(())
            }
        }

        impl $crate::codecs::base::MCDecode for $name {
            fn decode(src: &mut $crate::proto::packet_bytes::PacketBytes) -> anyhow::Result<Self> {
                Ok(Self {
                    $(
                        $fname: generics_macro::get_ident!(src, $ftype),
                    )*
                })
            }
        }
    };
}

#[macro_export]
macro_rules! create_packet {
    ($name:tt, $id:literal) => {
        paste::paste! {
            $crate::create_codec!($name);

            impl $crate::proto::packets::Packet for $name {
                const ID: i32 = $id;
            }
        }
    };
    ($name:tt, $id:literal, $($fname:tt => $ftype:ty),*) => {
        paste::paste! {
            $crate::create_codec!($name, $($fname => $ftype),*);

            impl $crate::proto::packets::Packet for $name {
                const ID: i32 = $id;
            }
        }
    };
}

#[macro_export]
macro_rules! quickpkt {
    ($name:tt, $id:literal) => {
        paste::paste! {
            pub mod $name {
                #[allow(unused_imports)]
                use super::*;

                $crate::create_packet!([<$name:camel>], $id);
            }
        }
    };
    ($name:tt, $id:literal, $($fname:tt => $ftype:ty),*) => {
        paste::paste! {
            pub mod $name {
                #[allow(unused_imports)]
                use super::*;

                $crate::create_packet!([<$name:camel>], $id, $($fname => $ftype),*);
            }
        }
    };
}

#[macro_export]
macro_rules! expect_packet {
    ($rd:expr, $name:tt) => {{
        let (id, mut payload) = $rd.read_packet().await?;
        if id == $name::ID {
            $name::decode(&mut payload)
        } else {
            anyhow::bail!(
                "Expected packet {} but instead got ID {id}",
                stringify!($name)
            );
        }
    }};
}
