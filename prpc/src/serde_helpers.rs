pub use prpc_serde_bytes::prpc_serde_bytes;

pub mod bytes_as_hex_str {
    use alloc::string::String;
    use alloc::vec::Vec;
    use serde::{Deserialize, Serialize};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("0x{}", hex_fmt::HexFmt(bytes)).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let hex_str = hex_str.trim_start_matches("0x");
        hex::decode(hex_str).map_err(serde::de::Error::custom)
    }
}

pub mod vec_bytes_as_hex_str {
    use alloc::string::String;
    use alloc::vec::Vec;
    use serde::{Deserialize, Serialize};

    #[allow(clippy::ptr_arg)]
    pub fn serialize<S>(bytes: &Vec<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_strs: Vec<String> = bytes
            .iter()
            .map(|bytes| format!("0x{}", hex_fmt::HexFmt(bytes)))
            .collect();
        hex_strs.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_strs: Vec<String> = Vec::deserialize(deserializer)?;
        hex_strs
            .into_iter()
            .map(|hex_str| {
                let hex_str = hex_str.trim_start_matches("0x");
                hex::decode(hex_str).map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

pub mod option_bytes_as_hex_str {
    use alloc::string::String;
    use alloc::vec::Vec;
    use serde::{Deserialize, Serialize};

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match bytes {
            Some(bytes) => format!("0x{}", hex_fmt::HexFmt(bytes)).serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str: Option<String> = Option::deserialize(deserializer)?;
        match hex_str {
            Some(hex_str) => {
                let hex_str = hex_str.trim_start_matches("0x");
                hex::decode(hex_str)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
            None => Ok(None),
        }
    }
}
