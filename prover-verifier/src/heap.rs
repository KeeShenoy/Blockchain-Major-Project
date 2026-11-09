//! Optional allocator-level accounting, enabled by the `alloc-profile` feature.
//!
//! This counts bytes requested through Rust's allocator. That is a different quantity from RSS,
//! which also includes code, libraries, the stack, and allocator overhead.

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct HeapPhase {
    pub phase: &'static str,
    /// Live bytes when the phase ended.
    pub live_bytes: usize,
    /// Highest live bytes reached during the phase.
    pub peak_bytes: usize,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct HeapReport {
    pub peak_bytes: usize,
    pub allocation_count: usize,
    pub allocated_total_bytes: usize,
    pub phases: Vec<HeapPhase>,
}

#[cfg(feature = "alloc-profile")]
mod tracking {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        sync::{
            atomic::{AtomicUsize, Ordering::Relaxed},
            Mutex,
        },
    };

    use super::{HeapPhase, HeapReport};

    static LIVE: AtomicUsize = AtomicUsize::new(0);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    static PHASE_PEAK: AtomicUsize = AtomicUsize::new(0);
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    static TOTAL: AtomicUsize = AtomicUsize::new(0);

    // Fixed capacity, so recording a phase does not itself allocate.
    const MAX_PHASES: usize = 16;
    const EMPTY: HeapPhase = HeapPhase {
        phase: "",
        live_bytes: 0,
        peak_bytes: 0,
    };
    static PHASES: Mutex<(usize, [HeapPhase; MAX_PHASES])> = Mutex::new((0, [EMPTY; MAX_PHASES]));

    struct TrackingAllocator;

    fn grew(bytes: usize) {
        let live = LIVE.fetch_add(bytes, Relaxed) + bytes;
        PEAK.fetch_max(live, Relaxed);
        PHASE_PEAK.fetch_max(live, Relaxed);
        TOTAL.fetch_add(bytes, Relaxed);
    }

    // SAFETY: every call forwards to `System` with its arguments unchanged.
    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            grew(layout.size());
            COUNT.fetch_add(1, Relaxed);
            System.alloc(layout)
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            LIVE.fetch_sub(layout.size(), Relaxed);
            System.dealloc(pointer, layout)
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if new_size > layout.size() {
                grew(new_size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Relaxed);
            }
            COUNT.fetch_add(1, Relaxed);
            System.realloc(pointer, layout, new_size)
        }
    }

    #[global_allocator]
    static ALLOCATOR: TrackingAllocator = TrackingAllocator;

    /// Closes the current phase under `phase`, and starts the next one. Only the first proof in a
    /// process is recorded, because later phases would overflow the fixed table.
    pub fn checkpoint(phase: &'static str) {
        let live = LIVE.load(Relaxed);
        let peak = PHASE_PEAK.swap(live, Relaxed);
        let mut phases = PHASES.lock().unwrap();
        let (count, slots) = &mut *phases;
        if *count < MAX_PHASES {
            slots[*count] = HeapPhase {
                phase,
                live_bytes: live,
                peak_bytes: peak,
            };
            *count += 1;
        }
    }

    /// Peak live bytes so far, for reading a whole-pipeline peak at a chosen point.
    pub fn peak_bytes() -> usize {
        PEAK.load(Relaxed)
    }

    pub fn report() -> Option<HeapReport> {
        let phases = PHASES.lock().unwrap();
        Some(HeapReport {
            peak_bytes: PEAK.load(Relaxed),
            allocation_count: COUNT.load(Relaxed),
            allocated_total_bytes: TOTAL.load(Relaxed),
            phases: phases.1[..phases.0].to_vec(),
        })
    }
}

#[cfg(feature = "alloc-profile")]
pub use tracking::{checkpoint, peak_bytes, report};

#[cfg(not(feature = "alloc-profile"))]
#[inline(always)]
pub fn checkpoint(_phase: &'static str) {}

#[cfg(not(feature = "alloc-profile"))]
pub fn peak_bytes() -> usize {
    0
}

#[cfg(not(feature = "alloc-profile"))]
pub fn report() -> Option<HeapReport> {
    None
}
