//! Process memory as reported by the kernel.
//!
//! `getrusage(RUSAGE_SELF).ru_maxrss` is deliberately not used. Linux carries the parent's peak RSS
//! across `exec`, so under `cargo run` it reports cargo's peak (about 35 MB) rather than this
//! program's (about 3.7 MB). `VmHWM` belongs to the current address space and has no such floor.

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct ProcessMemory {
    /// Peak resident set of this address space (`VmHWM`).
    pub peak_rss_bytes: u64,
    /// Current resident set (`VmRSS`).
    pub rss_bytes: u64,
    /// Anonymous share of the current resident set: heap, stack, private data (`RssAnon`).
    pub rss_anon_bytes: u64,
    /// File-backed share of the current resident set: mapped code and libraries (`RssFile`).
    pub rss_file_bytes: u64,
}

/// Reads `/proc/self/status`. Returns `None` where procfs is unavailable.
pub fn read_process_memory() -> Option<ProcessMemory> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| -> Option<u64> {
        let line = status.lines().find(|line| line.starts_with(name))?;
        let kib: u64 = line[name.len()..]
            .trim()
            .strip_suffix("kB")?
            .trim()
            .parse()
            .ok()?;
        Some(kib * 1024)
    };
    Some(ProcessMemory {
        peak_rss_bytes: field("VmHWM:")?,
        rss_bytes: field("VmRSS:")?,
        rss_anon_bytes: field("RssAnon:")?,
        rss_file_bytes: field("RssFile:")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_is_at_least_current() {
        let memory = read_process_memory().expect("tests run on Linux");
        assert!(memory.peak_rss_bytes >= memory.rss_bytes);
        assert!(memory.rss_bytes > 0);
        assert!(memory.rss_anon_bytes > 0);
    }
}
