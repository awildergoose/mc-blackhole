use crate::{
    codecs::base::{MCDecode, MCEncode},
    proto::packet_bytes::PacketBytes,
};

pub mod config;
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
macro_rules! create_codec {
    ($name:tt, $($fname:tt => $ftype:ty),*) => {
        use $crate::{
            proto::{packet_bytes::PacketBytes},
            codecs::base::{MCEncode, MCDecode}
        };
        use generics_macro::{put_ident, get_ident};

        #[derive(Clone)]
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

        impl MCEncode for $name {
            fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()> {
                $(
                    put_ident!(dst, $ftype, self.$fname.clone());
                )*
                Ok(())
            }
        }

        impl MCDecode for $name {
            fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
                Ok(Self {
                    $(
                        $fname: get_ident!(src, $ftype),
                    )*
                })
            }
        }
    };
}

#[macro_export]
macro_rules! create_packet {
    ($name:tt, $id: literal, $($fname:tt => $ftype:ty),*) => {
        use $crate::proto::packets::Packet;
        use paste::paste;

        paste! {
            $crate::create_codec!($name, $($fname => $ftype),*);

            impl Packet for $name {
                const ID: i32 = $id;
            }
        }
    };
}

#[macro_export]
macro_rules! quickpkt {
    ($name:tt, $id: literal, $($fname:tt => $ftype:ty),*) => {
        paste::paste! {
            pub mod $name {
                #[allow(unused_imports)]
                use super::*;

                $crate::create_packet!([<$name:camel>], $id, $($fname => $ftype),*);
            }
        }
    };
}
