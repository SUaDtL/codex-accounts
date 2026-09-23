use crate::{DataError, MAX_JSON_DEPTH, MAX_RESOURCE_BYTES};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::fmt;

struct Check<'a> {
    containers: usize,
    reason: &'a Cell<DataError>,
}

impl Check<'_> {
    fn container<E: de::Error>(&self) -> Result<(), E> {
        if self.containers >= MAX_JSON_DEPTH {
            self.reason.set(DataError::DepthLimit);
            return Err(E::custom("depth limit"));
        }
        Ok(())
    }
    fn child(&self) -> Check<'_> {
        Check {
            containers: self.containers + 1,
            reason: self.reason,
        }
    }
}

impl<'de> DeserializeSeed<'de> for Check<'_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Check<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded JSON")
    }
    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        self.container()?;
        while seq.next_element_seed(self.child())?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        self.container()?;
        // Compare decoded keys, so "a" and "\u0061" are duplicates too.
        // This temporary parser allocation is not guaranteed to be zeroized.
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                self.reason.set(DataError::DuplicateKey);
                return Err(de::Error::custom("duplicate key"));
            }
            map.next_value_seed(self.child())?;
        }
        Ok(())
    }
}

/// Validate structure without decoding into a last-key-wins object or rewriting
/// the retained bytes. The caller still needs a reviewed auth-mode/schema adapter.
/// Depth counts object/array containers, including the root object.
pub fn validate_json_object(bytes: &[u8]) -> Result<(), DataError> {
    if bytes.len() > MAX_RESOURCE_BYTES {
        return Err(DataError::InputLimit);
    }
    if std::str::from_utf8(bytes).is_err()
        || bytes.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'{')
    {
        return Err(DataError::InvalidJson);
    }
    let reason = Cell::new(DataError::InvalidJson);
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    Check {
        containers: 0,
        reason: &reason,
    }
    .deserialize(&mut parser)
    .map_err(|_| reason.get())?;
    parser.end().map_err(|_| DataError::InvalidJson)
}
