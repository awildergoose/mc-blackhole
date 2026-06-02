use crate::{codecs::array::Array, create_codec, quickpkt};

create_codec!(KnownPack, namespace => String, id => String, version => String);

quickpkt!(
    sc_update_enabled_features,
    0x0C, features => Array<String>
);
quickpkt!(
    sc_select_known_packs,
    0x0E, features => Array<KnownPack>
);
