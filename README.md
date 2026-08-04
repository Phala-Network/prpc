# Phala Remote Procedure Call Library

This library provides a set of functions to interact with Phala Network's Offchain Workers.

## Wire format for empty messages

`google.protobuf.Empty` maps to the Rust unit type and is encoded as an **empty body** in
both codecs:

| Message                 | protobuf body | JSON body    |
| ----------------------- | ------------- | ------------ |
| `google.protobuf.Empty` | empty         | empty        |
| any other message       | encoded bytes | encoded JSON |

The protobuf codec has always done this, since a unit encodes to zero bytes. Before
`prpc-build` 0.7.0 the JSON codec instead emitted the literal `null`, which forced clients
to special-case a body that is neither a message nor an empty response.

Transports decoding a JSON body should use `prpc::codec::decode_json_from_slice`, which
maps an empty body back to the unit type. An empty body for any other message type remains
an error.
