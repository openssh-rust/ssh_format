# ssh_mux_format

[![Rust](https://github.com/NobodyXu/ssh_format/actions/workflows/rust.yml/badge.svg)](https://github.com/NobodyXu/ssh_format/actions/workflows/rust.yml)

[![crate.io downloads](https://img.shields.io/crates/d/ssh_format)](https://crates.io/crates/ssh_format)

[![crate.io version](https://img.shields.io/crates/v/ssh_format)](https://crates.io/crates/ssh_format)

[![docs](https://docs.rs/ssh_format/badge.svg)](https://docs.rs/ssh_format)

Data format used to communicate with openssh mux server.

Format details:
 - All integers are encoded in big endian;
 - Boolean are encoded as `u32` according to [here][1];
 - `char` are encoded as `u32`;
 - Strings and bytes are encoded as length(`u32`) + content, same as [`sshbuf_put_string`];
 - `Option::None` are omitted while `Option::Some(v)` has the same encoding as `v` since
   openssh mux protocol allows optional parameter at the end of the message;
 - struct/tuple are encoded as-is, unit struct/tuple are omitted;
 - sequence are encoded as if it is a tuple according to [here][0], thus it cannot be
   deserialized;
 - Variant is encoded as index(`u32`) + content encoded as-is (it is expected to manually
   implement `Serialize` and `Deserialize` to ensure the `variant_index` is the one
   you expected);
 - Serializing/Deserializing map is unsupported;

[`sshbuf_put_string`]: https://github.com/openssh/openssh-portable/blob/2dc328023f60212cd29504fc05d849133ae47355/sshbuf-getput-basic.c#L514
[0]: https://github.com/openssh/openssh-portable/blob/19b3d846f06697c85957ab79a63454f57f8e22d6/mux.c#L1906
[1]: https://github.com/openssh/openssh-portable/blob/19b3d846f06697c85957ab79a63454f57f8e22d6/mux.c#L1897

## Feature
 - `is_human_readable` enables `Serializer::is_human_readable` and
   `Deserializer::is_human_readable`.
