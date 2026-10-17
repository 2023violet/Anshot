//! UI-independent editor session and asynchronous-result guard.
//! SR-V2-01 foundation; actual native canvas commands belong to SR-V2-02.
#![forbid(unsafe_code)]

use shareready_core::{Region, MAX_PIXELS};

const MAX_MASKS: usize = 1024;
const HISTORY_LIMIT: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub document_id: u64,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    Review,
    Copy,
    Save,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub stamp: Stamp,
    pub operation_id: u64,
    pub kind: OperationKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidImageDimensions,
    NoEffectiveMask,
    MaskLimit,
    InvalidMaskIndex,
    EmptyHistory,
    OperationPending,
    OutputNotReviewed,
    IdentityOverflow,
}

#[derive(Debug)]
pub struct Editor {
    stamp: Stamp,
    width: u32,
    height: u32,
    masks: Vec<Region>,
    undo: Vec<Vec<Region>>,
    redo: Vec<Vec<Region>>,
    reviewed: Option<Stamp>,
    pending: Option<Ticket>,
    last_delivery: Option<(Ticket, bool)>,
    next_op: u64,
}

impl Editor {
    pub fn new(width: u32, height: u32) -> Result<Self, EditError> {
        Self::validate_dimensions(width, height)?;
        Ok(Self {
            stamp: Stamp { document_id: 1, revision: 0 },
            width,
            height,
            masks: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            reviewed: None,
            pending: None,
            last_delivery: None,
            next_op: 1,
        })
    }

    fn validate_dimensions(width: u32, height: u32) -> Result<(), EditError> {
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
            return Err(EditError::InvalidImageDimensions);
        }
        Ok(())
    }

    pub fn stamp(&self) -> Stamp { self.stamp }
    pub fn masks(&self) -> &[Region] { &self.masks }
    pub fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }
    pub fn is_reviewed(&self) -> bool { self.reviewed == Some(self.stamp) }
    pub fn last_delivery(&self) -> Option<(Ticket, bool)> { self.last_delivery }

    /// New document identity invalidates *all* previous work, even at revision zero.
    pub fn replace_document(&mut self, width: u32, height: u32) -> Result<(), EditError> {
        Self::validate_dimensions(width, height)?;
        let document_id = self.stamp.document_id.checked_add(1)
            .ok_or(EditError::IdentityOverflow)?;
        self.width = width;
        self.height = height;
        self.stamp = Stamp { document_id, revision: 0 };
        self.masks.clear();
        self.undo.clear();
        self.redo.clear();
        self.invalidate();
        Ok(())
    }

    fn invalidate(&mut self) {
        self.reviewed = None;
        self.pending = None;
        self.last_delivery = None;
    }

    fn change(&mut self, next: Vec<Region>) -> Result<(), EditError> {
        let revision = self.stamp.revision.checked_add(1)
            .ok_or(EditError::IdentityOverflow)?;
        let previous = std::mem::replace(&mut self.masks, next);
        if self.undo.len() == HISTORY_LIMIT { self.undo.remove(0); }
        self.undo.push(previous);
        self.redo.clear();
        self.stamp.revision = revision;
        self.invalidate();
        Ok(())
    }

    /// Normalize to pixel-aligned half-open coordinates. Reject zero or out-of-image area.
    pub fn add_mask(&mut self, candidate: Region) -> Result<usize, EditError> {
        if self.masks.len() == MAX_MASKS { return Err(EditError::MaskLimit); }
        let (l, t, r, b) = candidate.clipped(self.width, self.height)
            .ok_or(EditError::NoEffectiveMask)?;
        let mut next = self.masks.clone();
        next.push(Region::new(i64::from(l), i64::from(t), i64::from(r), i64::from(b)));
        self.change(next)?;
        Ok(self.masks.len() - 1)
    }

    pub fn remove_mask(&mut self, index: usize) -> Result<(), EditError> {
        if index >= self.masks.len() { return Err(EditError::InvalidMaskIndex); }
        let mut next = self.masks.clone();
        next.remove(index);
        self.change(next)
    }

    pub fn undo(&mut self) -> Result<(), EditError> {
        let revision = self.stamp.revision.checked_add(1)
            .ok_or(EditError::IdentityOverflow)?;
        let old = self.undo.pop().ok_or(EditError::EmptyHistory)?;
        let previous = std::mem::replace(&mut self.masks, old);
        self.redo.push(previous);
        self.stamp.revision = revision;
        self.invalidate();
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), EditError> {
        let revision = self.stamp.revision.checked_add(1)
            .ok_or(EditError::IdentityOverflow)?;
        let next = self.redo.pop().ok_or(EditError::EmptyHistory)?;
        let previous = std::mem::replace(&mut self.masks, next);
        if self.undo.len() == HISTORY_LIMIT { self.undo.remove(0); }
        self.undo.push(previous);
        self.stamp.revision = revision;
        self.invalidate();
        Ok(())
    }

    pub fn begin(&mut self, kind: OperationKind) -> Result<Ticket, EditError> {
        if self.pending.is_some() { return Err(EditError::OperationPending); }
        match kind {
            OperationKind::Review => {
                if !self.masks.iter().any(|mask| mask.clipped(self.width, self.height).is_some()) {
                    return Err(EditError::NoEffectiveMask);
                }
            }
            OperationKind::Copy | OperationKind::Save => {
                if !self.is_reviewed() { return Err(EditError::OutputNotReviewed); }
            }
        }
        let next_op = self.next_op.checked_add(1)
            .ok_or(EditError::IdentityOverflow)?;
        let ticket = Ticket { stamp: self.stamp, operation_id: self.next_op, kind };
        self.next_op = next_op;
        self.pending = Some(ticket);
        self.last_delivery = None;
        Ok(ticket)
    }

    /// Only the latest exact document/revision/operation may update user-visible status.
    /// A true completion means the *caller* verified a real OS operation; the editor
    /// deliberately cannot turn an attempted Copy/Save into a success by itself.
    pub fn finish(&mut self, ticket: Ticket, succeeded: bool) -> bool {
        if self.pending != Some(ticket) || ticket.stamp != self.stamp { return false; }
        self.pending = None;
        match ticket.kind {
            OperationKind::Review => {
                self.reviewed = if succeeded { Some(self.stamp) } else { None };
            }
            OperationKind::Copy | OperationKind::Save => {
                self.last_delivery = Some((ticket, succeeded));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor { Editor::new(1200, 800).unwrap() }
    fn region() -> Region { Region::new(84, 154, 166, 237) }
    fn reviewed(e: &mut Editor) {
        e.add_mask(region()).unwrap();
        let t = e.begin(OperationKind::Review).unwrap();
        assert!(e.finish(t, true));
    }

    #[test]
    fn reject_invalid_dimensions_and_zero_effective_mask() {
        assert_eq!(Editor::new(0, 8).unwrap_err(), EditError::InvalidImageDimensions);
        assert_eq!(Editor::new(6000, 6000).unwrap_err(), EditError::InvalidImageDimensions);
        let mut e = editor();
        assert_eq!(e.begin(OperationKind::Review), Err(EditError::NoEffectiveMask));
        for region in [Region::new(0, 0, 0, 1), Region::new(-8, -8, -1, -1)] {
            assert_eq!(e.add_mask(region), Err(EditError::NoEffectiveMask));
        }
        assert!(e.masks().is_empty());
    }

    #[test]
    fn reverse_one_pixel_is_exact_and_clamped() {
        let mut e = editor();
        e.add_mask(Region::new(15, 16, 14, 15)).unwrap();
        assert_eq!(e.masks()[0], Region::new(14, 15, 15, 16));
        e.add_mask(Region::new(-5, -5, 10, 10)).unwrap();
        assert_eq!(e.masks()[1], Region::new(0, 0, 10, 10));
    }

    #[test]
    fn undo_redo_do_not_reuse_an_old_revision() {
        let mut e = editor();
        e.add_mask(region()).unwrap();
        let s = e.stamp();
        e.undo().unwrap();
        assert!(e.masks().is_empty());
        e.redo().unwrap();
        assert_eq!(e.masks().len(), 1);
        assert!(e.stamp().revision > s.revision);
        e.undo().unwrap();
        e.add_mask(Region::new(1, 1, 2, 2)).unwrap();
        assert_eq!(e.redo(), Err(EditError::EmptyHistory));
    }

    #[test]
    fn late_review_cannot_validate_a_changed_document() {
        let mut e = editor();
        e.add_mask(region()).unwrap();
        let old = e.begin(OperationKind::Review).unwrap();
        e.add_mask(Region::new(1, 1, 2, 2)).unwrap();
        assert!(!e.finish(old, true));
        assert!(!e.is_reviewed());
        let current = e.begin(OperationKind::Review).unwrap();
        assert!(e.finish(current, true));
        assert!(e.is_reviewed());
    }

    #[test]
    fn old_copy_cannot_claim_new_document_success() {
        let mut e = editor();
        reviewed(&mut e);
        let stale = e.begin(OperationKind::Copy).unwrap();
        e.replace_document(300, 200).unwrap();
        e.add_mask(Region::new(1, 1, 2, 2)).unwrap();
        let fresh = e.begin(OperationKind::Review).unwrap();
        assert!(!e.finish(stale, true));
        assert!(!e.is_reviewed());
        assert_eq!(e.begin(OperationKind::Copy), Err(EditError::OperationPending));
        assert!(e.finish(fresh, true));
        assert!(e.is_reviewed());
        let copy = e.begin(OperationKind::Copy).unwrap();
        assert_ne!(copy, stale);
        assert!(e.finish(copy, false));
        assert_eq!(e.last_delivery(), Some((copy, false)));
    }

    #[test]
    fn operation_attempts_are_single_use_and_ticket_bound() {
        let mut e = editor();
        reviewed(&mut e);
        let save = e.begin(OperationKind::Save).unwrap();
        assert_eq!(e.begin(OperationKind::Copy), Err(EditError::OperationPending));
        assert!(e.finish(save, true));
        assert!(!e.finish(save, true));
        let next = e.begin(OperationKind::Save).unwrap();
        assert!(!e.finish(save, true));
        assert!(e.finish(next, false));
        assert_eq!(e.last_delivery(), Some((next, false)));
    }

    #[test]
    fn unsuccessful_review_does_not_unlock_copy() {
        let mut e = editor();
        e.add_mask(region()).unwrap();
        let t = e.begin(OperationKind::Review).unwrap();
        assert!(e.finish(t, false));
        assert_eq!(e.begin(OperationKind::Copy), Err(EditError::OutputNotReviewed));
    }

    #[test]
    fn invalid_replace_is_transactional() {
        let mut e = editor();
        reviewed(&mut e);
        let before = e.stamp();
        assert_eq!(e.replace_document(0, 1), Err(EditError::InvalidImageDimensions));
        assert_eq!(e.stamp(), before);
        assert!(e.is_reviewed());
        assert_eq!(e.masks().len(), 1);
    }

    #[test]
    fn removing_mask_invalidates_review_and_honors_index() {
        let mut e = editor();
        reviewed(&mut e);
        assert_eq!(e.remove_mask(4), Err(EditError::InvalidMaskIndex));
        assert!(e.is_reviewed());
        e.remove_mask(0).unwrap();
        assert!(!e.is_reviewed());
        assert_eq!(e.begin(OperationKind::Review), Err(EditError::NoEffectiveMask));
    }
}
