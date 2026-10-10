use super::oracle_tests::bundle;
use super::*;
use crate::storage_support::work::{ReadFence, WorkImage};
use std::cell::{Cell, RefCell};
struct Raw {
    fence: RefCell<ReadFence>,
    header: RefCell<MetadataHeader>,
    events: RefCell<Vec<JournalEntry>>,
    reads: Cell<usize>,
}
impl JournalRead for Raw {
    fn coherence(&self) -> ReadFence {
        self.fence.borrow().clone()
    }
    fn header(&self) -> Result<MetadataHeader> {
        Ok(self.header.borrow().clone())
    }
    fn entry(&self, p: u64) -> Result<Option<JournalEntry>> {
        self.reads.set(self.reads.get() + 1);
        Ok(self
            .events
            .borrow()
            .iter()
            .find(|e| e.event.position == p)
            .cloned())
    }
}
fn raw() -> (Raw, WorkImage) {
    let mut s = StorageState::new(StorageLimits {
        journal_rows: 1,
        ..Default::default()
    })
    .unwrap();
    s.bundle(&bundle("old", "test", true)).unwrap();
    let j = JournalImage::from_metadata(StorageMetadata::from_state(&s)).unwrap();
    let r = Raw {
        fence: RefCell::new(ReadFence::new()),
        header: RefCell::new(j.header().clone()),
        events: RefCell::new(
            j.events()
                .iter()
                .map(|e| JournalEntry {
                    event: e.clone(),
                    encoded_bytes: serde_json::to_vec(e).unwrap().len(),
                })
                .collect(),
        ),
        reads: Cell::new(0),
    };
    (r, WorkImage::from_ledger(s.work, s.retry_epochs).unwrap())
}
#[test]
fn exact_read_facts_reject_read_only_and_header_mutations_and_cross_context() {
    for case in 0..4 {
        let (r, w) = raw();
        let d = prepare_native_bundle(&r, &w, &bundle("new", "test", true)).unwrap();
        match case {
            0 => r.events.borrow_mut()[0].event.identity = "edited".into(),
            1 => {
                let mut p = r.header.borrow().parts();
                p.receipts += 1;
                *r.header.borrow_mut() = MetadataHeader::from_parts(p).unwrap();
            }
            2 => *r.fence.borrow_mut() = ReadFence::new(),
            _ => {
                let mut e = r.events.borrow()[0].clone();
                e.event.position = 2;
                r.events.borrow_mut().push(e);
            }
        }
        assert_eq!(d.validate(&r, &w), Err(Error::Conflict));
    }
}
#[test]
fn touched_byte_mismatch_is_rejected_and_changed_false_does_not_scan() {
    let (r, w) = raw();
    r.events.borrow_mut()[0].encoded_bytes += 1;
    assert!(matches!(
        prepare_native_bundle(&r, &w, &bundle("n", "test", true)),
        Err(Error::Storage)
    ));
    r.reads.set(0);
    assert!(prepare_native_bundle(&r, &w, &bundle("n", "test", false)).is_ok());
    assert_eq!(r.reads.get(), 0);
    assert_eq!(
        journal_page(&r, "test", None, 10, 1000),
        Err(Error::Storage)
    );
}
#[test]
fn work_only_publication_invalidates_combined_candidate() {
    let (r, mut w) = raw();
    let d = prepare_native_bundle(&r, &w, &bundle("n", "test", true)).unwrap();
    let update =
        crate::storage_support::work::prepare_update(&w, WorkUpdate::Claim { now: 1 }).unwrap();
    w.apply(update).unwrap();
    assert_eq!(d.validate(&r, &w), Err(Error::Conflict));
}

#[test]
fn cumulative_reader_budget_is_propagated_before_prefix_publication() {
    struct Budget {
        r: Raw,
        remaining: Cell<usize>,
    }
    impl JournalRead for Budget {
        fn coherence(&self) -> ReadFence {
            self.r.coherence()
        }
        fn header(&self) -> Result<MetadataHeader> {
            self.r.header()
        }
        fn entry(&self, p: u64) -> Result<Option<JournalEntry>> {
            let data = self.r.events.borrow();
            let e = data.iter().find(|e| e.event.position == p);
            let bytes = e.map_or(0, |e| e.encoded_bytes);
            self.remaining.set(
                self.remaining
                    .get()
                    .checked_sub(bytes)
                    .ok_or(Error::TooLarge)?,
            );
            Ok(e.cloned())
        }
    }
    let mut state = StorageState::new(StorageLimits {
        journal_bytes: 1000,
        ..Default::default()
    })
    .unwrap();
    for n in 0..5 {
        state
            .bundle(&bundle(&format!("old{n}"), "a", true))
            .unwrap();
    }
    let image = JournalImage::from_metadata(StorageMetadata::from_state(&state)).unwrap();
    let events: Vec<_> = image
        .events()
        .iter()
        .map(|e| JournalEntry {
            event: e.clone(),
            encoded_bytes: serde_json::to_vec(e).unwrap().len(),
        })
        .collect();
    let budget = events[0].encoded_bytes;
    let read = Budget {
        r: Raw {
            fence: RefCell::new(ReadFence::new()),
            header: RefCell::new(image.header().clone()),
            events: RefCell::new(events),
            reads: Cell::new(0),
        },
        remaining: Cell::new(budget),
    };
    let work = WorkImage::from_ledger(state.work.clone(), state.retry_epochs).unwrap();
    let mut b = bundle("large", "a", true);
    b.receipt.row.value = Some(json!({"text":"x".repeat(600)}));
    assert!(matches!(
        prepare_native_bundle(&read, &work, &b),
        Err(Error::TooLarge)
    ));
    assert_eq!(read.r.header.borrow().parts().head, 5);
    read.remaining.set(budget);
    assert_eq!(
        journal_page(&read, "a", None, 10, 10000),
        Err(Error::TooLarge)
    );
}
