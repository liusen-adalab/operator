pub extern crate derive_more;
pub extern crate flaken;

#[macro_export]
macro_rules! id_new_type {
    ($type_name:ident) => {
        #[derive(
            Debug,
            PartialEq,
            PartialOrd,
            Eq,
            Hash,
            Clone,
            Copy,
            $crate::macros::id_wraper::derive_more::From,
            $crate::macros::id_wraper::derive_more::Display,
            $crate::macros::id_wraper::derive_more::FromStr,
            $crate::macros::id_wraper::derive_more::Into,
        )]
        pub struct $type_name(pub i64);

        impl $type_name {
            pub fn next_id() -> Self {
                use flaken::Flaken;
                use std::sync::{Mutex, OnceLock};
                use $crate::macros::id_wraper::flaken;
                static USER_ID_GENERATOR: OnceLock<Mutex<Flaken>> = OnceLock::new();
                let f = USER_ID_GENERATOR.get_or_init(|| {
                    let ip = utils::process::get_local_ip_u32();
                    let f = flaken::Flaken::default();
                    let f = f.node(ip as u64);
                    Mutex::new(f)
                });
                let mut lock = f.lock().unwrap();
                Self(lock.next() as i64)
            }
        }

        impl serde::Serialize for $type_name {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> serde::Deserialize<'de> for $type_name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let id = String::deserialize(deserializer)?;
                let id = id.parse().map_err(serde::de::Error::custom)?;
                Ok(Self(id))
                // #[derive(serde::Deserialize)]
                // #[serde(untagged)]
                // enum StringOrInt {
                //     String(String),
                //     Int(i64),
                // }
                // let id = StringOrInt::deserialize(deserializer)?;
                // match id {
                //     StringOrInt::String(id) => {
                //         let id = id.parse().map_err(serde::de::Error::custom)?;
                //         Ok(Self(id))
                //     }
                //     StringOrInt::Int(id) => Ok(Self(id)),
                // }
            }
        }
    };
}
