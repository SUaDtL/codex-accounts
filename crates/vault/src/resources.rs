use crate::{validate_json_object, DataError, MAX_RESOURCES, MAX_RESOURCE_BYTES, MAX_SET_BYTES};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A schema-local slot, never a filename or a qualified resource identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceId(u8);
impl ResourceId {
    pub fn new(slot: u8) -> Result<Self, DataError> {
        if slot as usize >= MAX_RESOURCES {
            return Err(DataError::InvalidId);
        }
        Ok(Self(slot))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceShape {
    JsonObject,
    Opaque,
}

/// Exact in-memory input with explicit absence. Deliberately not Clone/Serialize.
/// The fill on drop is best effort, not a verified secure-erasure primitive.
pub struct Resource {
    id: ResourceId,
    bytes: Option<Vec<u8>>,
}
impl fmt::Debug for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Resource([REDACTED])")
    }
}
impl Drop for Resource {
    fn drop(&mut self) {
        if let Some(bytes) = &mut self.bytes {
            bytes.fill(0);
        }
    }
}
impl Resource {
    pub fn absent(id: ResourceId) -> Self {
        Self { id, bytes: None }
    }
    pub fn present(id: ResourceId, mut bytes: Vec<u8>) -> Result<Self, DataError> {
        if bytes.len() > MAX_RESOURCE_BYTES {
            bytes.fill(0);
            return Err(DataError::InputLimit);
        }
        Ok(Self {
            id,
            bytes: Some(bytes),
        })
    }
    pub fn id(&self) -> ResourceId {
        self.id
    }
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}

/// In-memory set, not an auth schema, encrypted envelope, or authority to read/write.
/// Rules are structural inputs only; production must derive them from qualification.
pub struct CredentialSet {
    resources: BTreeMap<ResourceId, Resource>,
}
impl fmt::Debug for CredentialSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CredentialSet([REDACTED])")
    }
}
impl CredentialSet {
    pub fn new(
        rules: &[(ResourceId, ResourceShape, bool)],
        resources: Vec<Resource>,
    ) -> Result<Self, DataError> {
        if rules.is_empty() || rules.len() > MAX_RESOURCES || resources.len() > MAX_RESOURCES {
            return Err(DataError::InputLimit);
        }
        let expected: BTreeSet<_> = rules.iter().map(|r| r.0).collect();
        if expected.len() != rules.len() {
            return Err(DataError::DuplicateResource);
        }
        let mut result = BTreeMap::new();
        let mut total: usize = 0;
        for resource in resources {
            total = total
                .checked_add(resource.as_bytes().map_or(0, <[u8]>::len))
                .ok_or(DataError::InputLimit)?;
            if total > MAX_SET_BYTES {
                return Err(DataError::InputLimit);
            }
            if result.insert(resource.id, resource).is_some() {
                return Err(DataError::DuplicateResource);
            }
        }
        if result.keys().copied().collect::<BTreeSet<_>>() != expected {
            return Err(DataError::ResourceSetMismatch);
        }
        for (id, shape, required) in rules {
            match result[id].as_bytes() {
                None if *required => return Err(DataError::RequiredResourceAbsent),
                Some(bytes) if *shape == ResourceShape::JsonObject => validate_json_object(bytes)?,
                _ => {}
            }
        }
        Ok(Self { resources: result })
    }
    pub fn resource(&self, id: ResourceId) -> Option<&Resource> {
        self.resources.get(&id)
    }
}
