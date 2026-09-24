//! Fixed bounded binary records. This is framing/serialization, not cryptography.
#![forbid(unsafe_code)]
use crate::{records::*, ProfileText, StorageError};
use codex_accounts_vault::Identity;
use zeroize::Zeroizing;

struct Writer(Zeroizing<Vec<u8>>);
impl Writer {
    fn new(magic: &[u8; 8]) -> Self {
        Self(Zeroizing::new(magic.to_vec()))
    }
    fn u8(&mut self, n: u8) {
        self.0.push(n);
    }
    fn u32(&mut self, n: u32) {
        self.0.extend_from_slice(&n.to_le_bytes());
    }
    fn u64(&mut self, n: u64) {
        self.0.extend_from_slice(&n.to_le_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
    fn id(&mut self, id: &Id) {
        self.0.extend_from_slice(id);
    }
    fn option_id(&mut self, id: Option<Id>) {
        self.u8(u8::from(id.is_some()));
        if let Some(id) = id {
            self.id(&id);
        }
    }
    fn blob(&mut self, b: &Blob) {
        self.u8(b.key.kind);
        self.id(&b.key.profile);
        self.id(&b.key.generation);
        self.u8(b.key.slot);
        self.u32(b.size);
        self.0.extend_from_slice(&b.hash);
    }
    fn optional(&mut self, b: Option<&Blob>) {
        self.u8(u8::from(b.is_some()));
        if let Some(b) = b {
            self.blob(b);
        }
    }
    fn blobs(&mut self, b: &[Blob]) {
        self.u32(b.len() as u32);
        for v in b {
            self.blob(v);
        }
    }
    fn generation(&mut self, g: &Generation, manifest: bool) {
        self.id(&g.profile);
        self.id(&g.id);
        self.option_id(g.parent);
        self.u32(g.schema);
        self.u64(g.captured);
        self.u32(g.rules.len() as u32);
        for r in &g.rules {
            self.u8(r.slot);
            self.u8(u8::from(r.json));
            self.u8(u8::from(r.required));
            self.optional(r.blob.as_ref());
        }
        if manifest {
            self.blob(&g.manifest);
        }
    }
    fn finish(self) -> Result<Zeroizing<Vec<u8>>, StorageError> {
        if self.0.len() > MAX_METADATA {
            Err(StorageError::InputLimit)
        } else {
            Ok(self.0)
        }
    }
}
struct Reader<'a> {
    b: &'a [u8],
    n: usize,
}
impl<'a> Reader<'a> {
    fn new(b: &'a [u8], magic: &[u8; 8]) -> Result<Self, StorageError> {
        if b.len() > MAX_METADATA || !b.starts_with(magic) {
            return Err(StorageError::Corrupt);
        }
        Ok(Self { b, n: 8 })
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], StorageError> {
        let end = self.n.checked_add(n).ok_or(StorageError::Corrupt)?;
        let b = self.b.get(self.n..end).ok_or(StorageError::Corrupt)?;
        self.n = end;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, StorageError> {
        Ok(self.take(1)?[0])
    }
    fn boolean(&mut self) -> Result<bool, StorageError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(StorageError::Corrupt),
        }
    }
    fn u32(&mut self) -> Result<u32, StorageError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| StorageError::Corrupt)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, StorageError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| StorageError::Corrupt)?,
        ))
    }
    fn count(&mut self, max: usize) -> Result<usize, StorageError> {
        let n = self.u32()? as usize;
        if n > max {
            Err(StorageError::InputLimit)
        } else {
            Ok(n)
        }
    }
    fn bytes(&mut self, max: usize) -> Result<&'a [u8], StorageError> {
        let n = self.count(max)?;
        self.take(n)
    }
    fn text(&mut self, max: usize) -> Result<String, StorageError> {
        Ok(std::str::from_utf8(self.bytes(max)?)
            .map_err(|_| StorageError::Corrupt)?
            .to_owned())
    }
    fn id(&mut self) -> Result<Id, StorageError> {
        let id: Id = self
            .take(16)?
            .try_into()
            .map_err(|_| StorageError::Corrupt)?;
        if id == [0; 16] {
            Err(StorageError::Corrupt)
        } else {
            Ok(id)
        }
    }
    fn option_id(&mut self) -> Result<Option<Id>, StorageError> {
        if self.boolean()? {
            Ok(Some(self.id()?))
        } else {
            Ok(None)
        }
    }
    fn blob(&mut self) -> Result<Blob, StorageError> {
        let key = Key {
            kind: self.u8()?,
            profile: self.id()?,
            generation: self.id()?,
            slot: self.u8()?,
        };
        key.validate()?;
        let size = self.u32()?;
        let hash = self
            .take(32)?
            .try_into()
            .map_err(|_| StorageError::Corrupt)?;
        if size < 66 || size as usize > MAX_FILE {
            return Err(StorageError::Corrupt);
        }
        Ok(Blob { key, size, hash })
    }
    fn optional(&mut self) -> Result<Option<Blob>, StorageError> {
        if self.boolean()? {
            Ok(Some(self.blob()?))
        } else {
            Ok(None)
        }
    }
    fn blobs(&mut self) -> Result<Vec<Blob>, StorageError> {
        let n = self.count(MAX_BLOBS)?;
        (0..n).map(|_| self.blob()).collect()
    }
    fn generation(&mut self) -> Result<Generation, StorageError> {
        let profile = self.id()?;
        let id = self.id()?;
        let parent = self.option_id()?;
        let schema = self.u32()?;
        let captured = self.u64()?;
        let n = self.count(16)?;
        let mut rules = Vec::with_capacity(n);
        for _ in 0..n {
            rules.push(Rule {
                slot: self.u8()?,
                json: self.boolean()?,
                required: self.boolean()?,
                blob: self.optional()?,
            });
        }
        let manifest = self.blob()?;
        Ok(Generation {
            profile,
            id,
            parent,
            schema,
            captured,
            rules,
            manifest,
        })
    }
    fn end(self) -> Result<(), StorageError> {
        if self.n == self.b.len() {
            Ok(())
        } else {
            Err(StorageError::Corrupt)
        }
    }
}

pub(crate) fn generation(
    g: &Generation,
    identity: &Identity,
) -> Result<Zeroizing<Vec<u8>>, StorageError> {
    let mut w = Writer::new(b"CAGEN001");
    w.generation(g, false);
    let (i, s, a) = identity.components();
    w.bytes(i.as_bytes());
    w.bytes(s.as_bytes());
    w.bytes(a.as_bytes());
    w.finish()
}
pub(crate) fn registry(r: &Registry) -> Result<Zeroizing<Vec<u8>>, StorageError> {
    r.validate()?;
    let mut w = Writer::new(b"CAREG001");
    w.u64(r.revision);
    w.u8(u8::from(r.active_known));
    w.option_id(r.active);
    w.u32(r.profiles.len() as u32);
    for p in &r.profiles {
        w.id(&p.id);
        w.bytes(p.label.0.as_bytes());
        w.bytes(p.domain.0.as_bytes());
        let (i, s, a) = p.identity.components();
        w.bytes(i.as_bytes());
        w.bytes(s.as_bytes());
        w.bytes(a.as_bytes());
        w.id(&p.latest);
        w.u64(p.created);
    }
    w.u32(r.generations.len() as u32);
    for g in &r.generations {
        w.generation(g, true);
    }
    w.u32(r.holds.len() as u32);
    for h in &r.holds {
        w.id(&h.id);
        w.u32(h.generations.len() as u32);
        for g in &h.generations {
            w.id(g);
        }
    }
    w.finish()
}
pub(crate) fn read_registry(bytes: &[u8]) -> Result<Registry, StorageError> {
    let mut d = Reader::new(bytes, b"CAREG001")?;
    let revision = d.u64()?;
    let active_known = d.boolean()?;
    let active = d.option_id()?;
    let n = d.count(MAX_PROFILES)?;
    let mut profiles = Vec::with_capacity(n);
    for _ in 0..n {
        let id = d.id()?;
        let label = ProfileText::new(d.text(320)?)?;
        let domain = ProfileText::new(d.text(320)?)?;
        // Guard partial decoded strings if a later field fails to parse.
        let i = Zeroizing::new(d.text(2048)?);
        let s = Zeroizing::new(d.text(2048)?);
        let a = Zeroizing::new(d.text(2048)?);
        let identity = Identity::new(i.to_string(), s.to_string(), a.to_string())?;
        profiles.push(Profile {
            id,
            label,
            domain,
            identity,
            latest: d.id()?,
            created: d.u64()?,
        });
    }
    let n = d.count(MAX_GENERATIONS)?;
    let mut generations = Vec::with_capacity(n);
    for _ in 0..n {
        generations.push(d.generation()?);
    }
    let n = d.count(MAX_GENERATIONS)?;
    let mut holds = Vec::with_capacity(n);
    for _ in 0..n {
        let id = d.id()?;
        let count = d.count(MAX_GENERATIONS)?;
        let mut generations = Vec::with_capacity(count);
        for _ in 0..count {
            generations.push(d.id()?);
        }
        holds.push(Hold { id, generations });
    }
    d.end()?;
    let r = Registry {
        revision,
        active_known,
        active,
        profiles,
        generations,
        holds,
    };
    r.validate()?;
    Ok(r)
}
pub(crate) fn state(s: &State) -> Result<Zeroizing<Vec<u8>>, StorageError> {
    s.validate()?;
    let mut w = Writer::new(b"CASTA001");
    w.u64(s.sequence);
    w.u8(u8::from(s.parent.is_some()));
    if let Some(p) = s.parent {
        w.0.extend_from_slice(&p);
    }
    w.optional(s.current.as_ref());
    w.optional(s.next.as_ref());
    w.blobs(&s.writes);
    w.blobs(&s.garbage);
    w.u8(u8::from(s.deleting));
    w.u32(s.inline.len() as u32);
    for (b, data) in &s.inline {
        w.blob(b);
        w.bytes(data);
    }
    w.finish()
}
pub(crate) fn read_state(bytes: &[u8]) -> Result<State, StorageError> {
    let mut d = Reader::new(bytes, b"CASTA001")?;
    let sequence = d.u64()?;
    let parent = if d.boolean()? {
        Some(d.take(32)?.try_into().map_err(|_| StorageError::Corrupt)?)
    } else {
        None
    };
    let current = d.optional()?;
    let next = d.optional()?;
    let writes = d.blobs()?;
    let garbage = d.blobs()?;
    let deleting = d.boolean()?;
    let n = d.count(2)?;
    let mut inline = Vec::with_capacity(n);
    for _ in 0..n {
        inline.push((d.blob()?, d.bytes(MAX_METADATA + 66)?.to_vec()));
    }
    d.end()?;
    let s = State {
        sequence,
        parent,
        current,
        next,
        writes,
        garbage,
        deleting,
        inline,
    };
    s.validate()?;
    Ok(s)
}
