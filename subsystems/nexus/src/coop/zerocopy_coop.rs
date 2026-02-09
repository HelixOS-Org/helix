// SPDX-License-Identifier: GPL-2.0
//! Coop zerocopy — cooperative zero-copy transfer coordination

extern crate alloc;

/// Zerocopy coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZerocopyCoopEvent { PageShare, SpliceChain, SendfileCoord, MmapTransfer }

/// Zerocopy coop record
#[derive(Debug, Clone)]
pub struct ZerocopyCoopRecord {
    pub event: ZerocopyCoopEvent,
    pub pages: u32,
    pub bytes: u64,
    pub participants: u32,
}

impl ZerocopyCoopRecord {
    pub fn new(event: ZerocopyCoopEvent) -> Self { Self { event, pages: 0, bytes: 0, participants: 0 } }
}

/// Zerocopy coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ZerocopyCoopStats { pub total_events: u64, pub page_shares: u64, pub splices: u64, pub bytes_saved: u64 }

/// Main coop zerocopy
#[derive(Debug)]
pub struct CoopZerocopy { pub stats: ZerocopyCoopStats }

impl CoopZerocopy {
    pub fn new() -> Self { Self { stats: ZerocopyCoopStats { total_events: 0, page_shares: 0, splices: 0, bytes_saved: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &ZerocopyCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            ZerocopyCoopEvent::PageShare | ZerocopyCoopEvent::MmapTransfer => self.stats.page_shares += 1,
            ZerocopyCoopEvent::SpliceChain | ZerocopyCoopEvent::SendfileCoord => self.stats.splices += 1,
        }
        self.stats.bytes_saved += rec.bytes;
    }
}
