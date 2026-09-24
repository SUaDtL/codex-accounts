//! Explicit CAREG002 extension. CAREG001 reads without rewriting; old readers
//! reject the new magic. Version selection and graph checks precede all use.
#![forbid(unsafe_code)]
use super::{Reader, Writer};
use crate::{journal::*, records::*, StorageError};
use codex_accounts_core::{CredentialAcceptance, DesktopLaunch};

fn reference(w: &mut Writer, g: GenerationRef) {
    w.id(&g.profile);
    w.id(&g.generation);
}
fn read_reference(r: &mut Reader<'_>) -> Result<GenerationRef, StorageError> {
    Ok(GenerationRef {
        profile: r.id()?,
        generation: r.id()?,
    })
}
fn marks(w: &mut Writer, v: &[Mark]) {
    w.u32(v.len() as u32);
    for m in v {
        w.u8(m.slot);
        w.u8(u8::from(m.present));
        w.0.extend_from_slice(&m.tag);
    }
}
fn read_marks(r: &mut Reader<'_>) -> Result<Vec<Mark>, StorageError> {
    let mut result = Vec::new();
    for _ in 0..r.count(16)? {
        result.push(Mark {
            slot: r.u8()?,
            present: r.boolean()?,
            tag: r.take(32)?.try_into().map_err(|_| StorageError::Corrupt)?,
        });
    }
    Ok(result)
}
fn failure(r: &mut Reader<'_>) -> Result<Option<Failure>, StorageError> {
    match r.u8()? {
        0 => Ok(None),
        n => Ok(Some(Failure::parse(n)?)),
    }
}
pub(super) fn write(w: &mut Writer, j: &Journal) {
    w.id(&j.id);
    w.0.extend_from_slice(&j.binding);
    w.u64(j.created);
    w.u64(j.updated);
    reference(w, j.source);
    reference(w, j.target);
    reference(w, j.original_target);
    w.u8(j.phase as u8);
    for b in [j.source_saved, j.online, j.cancel, j.committed, j.cleaned] {
        w.u8(u8::from(b));
    }
    for bits in [
        j.staging_intent,
        j.staging_done,
        j.write_intent,
        j.write_done,
        j.restore_intent,
        j.restore_done,
    ] {
        w.u32(bits as u32);
    }
    w.u8(j.helper as u8);
    w.u8(j.failure.map_or(0, |e| e as u8));
    w.u8(j.restoration_failure.map_or(0, |e| e as u8));
    w.u8(match j.acceptance {
        CredentialAcceptance::Unknown => 0,
        CredentialAcceptance::Accepted => 1,
        CredentialAcceptance::Rejected => 2,
        CredentialAcceptance::ObservationUnavailable => 3,
    });
    w.u8(match j.launch {
        DesktopLaunch::NotRequested => 0,
        DesktopLaunch::Opened => 1,
        DesktopLaunch::Failed => 2,
    });
    marks(w, &j.source_marks);
    marks(w, &j.target_marks);
    w.u32(j.evidence.len() as u32);
    for e in &j.evidence {
        w.id(&e.id);
        w.u32(e.resources.len() as u32);
        for r in &e.resources {
            w.u8(r.slot);
            w.optional(r.blob.as_ref());
        }
    }
}
pub(super) fn read(r: &mut Reader<'_>) -> Result<Journal, StorageError> {
    let id = r.id()?;
    let binding = r.take(32)?.try_into().map_err(|_| StorageError::Corrupt)?;
    let created = r.u64()?;
    let updated = r.u64()?;
    let source = read_reference(r)?;
    let target = read_reference(r)?;
    let original_target = read_reference(r)?;
    let phase = SwitchPhase::parse(r.u8()?)?;
    let source_saved = r.boolean()?;
    let online = r.boolean()?;
    let cancel = r.boolean()?;
    let committed = r.boolean()?;
    let cleaned = r.boolean()?;
    let mut masks = [0u16; 6];
    for m in &mut masks {
        *m = u16::try_from(r.u32()?).map_err(|_| StorageError::Corrupt)?;
    }
    let helper = match r.u8()? {
        0 => Helper::Never,
        1 => Helper::MayRun,
        2 => Helper::Reaped,
        _ => return Err(StorageError::Corrupt),
    };
    let failure = failure(r)?;
    let restoration_failure = self::failure(r)?;
    let acceptance = match r.u8()? {
        0 => CredentialAcceptance::Unknown,
        1 => CredentialAcceptance::Accepted,
        2 => CredentialAcceptance::Rejected,
        3 => CredentialAcceptance::ObservationUnavailable,
        _ => return Err(StorageError::Corrupt),
    };
    let launch = match r.u8()? {
        0 => DesktopLaunch::NotRequested,
        1 => DesktopLaunch::Opened,
        2 => DesktopLaunch::Failed,
        _ => return Err(StorageError::Corrupt),
    };
    let source_marks = read_marks(r)?;
    let target_marks = read_marks(r)?;
    let mut evidence = Vec::new();
    for _ in 0..r.count(MAX_EVIDENCE)? {
        let id = r.id()?;
        let mut resources = Vec::new();
        for _ in 0..r.count(16)? {
            resources.push(Rule {
                slot: r.u8()?,
                json: false,
                required: false,
                blob: r.optional()?,
            });
        }
        evidence.push(Evidence { id, resources });
    }
    Ok(Journal {
        id,
        binding,
        created,
        updated,
        source,
        target,
        original_target,
        phase,
        source_saved,
        online,
        cancel,
        committed,
        cleaned,
        staging_intent: masks[0],
        staging_done: masks[1],
        write_intent: masks[2],
        write_done: masks[3],
        restore_intent: masks[4],
        restore_done: masks[5],
        helper,
        failure,
        restoration_failure,
        acceptance,
        launch,
        source_marks,
        target_marks,
        evidence,
    })
}
