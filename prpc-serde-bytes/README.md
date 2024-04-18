# prpc_serde_bytes

`prpc_serde_bytes` is a Rust procedural macro designed to work with structs that utilize both Serde and Prost crate attributes. Specifically, this macro targets struct fields marked with `#[prost(bytes = "vec")]` and automatically adds `#[serde(with = "as_bytes")]` to them, enabling custom serialization behavior as specified.

## Usage

Add `prpc_serde_bytes` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
prpc-serde-bytes = "0.1.0"
```

## Example

```rust
use prpc_serde_bytes::prpc_serde_bytes;

#[prpc_serde_bytes("::hexed_bytes")]
#[derive(Serialize, Deserialize, prost::Message)]
pub struct Message {
    #[prost(bytes = "vec", tag = "1")]
    pub encoded: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub timestamp: u64,
}
```

Which will generate the following code:

```rust
#[derive(Serialize, Deserialize, prost::Message)]
pub struct Message {
    #[prost(bytes = "vec", tag = "1")]
    #[serde(with = "::hexed_bytes")]
    pub encoded: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub timestamp: u64,
}
```