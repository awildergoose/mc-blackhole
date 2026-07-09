use crate::{
    codecs::array::{Array, RemainingArray},
    create_codec, quickpkt,
};

create_codec!(KnownPack, namespace => String, id => String, version => String);

quickpkt!(
    sc_update_enabled_features,
    0x0C, features => Array<String>
);
quickpkt!(
    sc_select_known_packs,
    0x0E, features => Array<KnownPack>
);
quickpkt!(sc_registries, 0x07, raw => RemainingArray<u8>);
quickpkt!(sc_tags, 0x0D, raw => RemainingArray<u8>);
quickpkt!(sc_finish_configuration, 0x03);
