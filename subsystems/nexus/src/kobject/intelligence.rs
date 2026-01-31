//! Kobject Intelligence
//!
//! Central coordinator for kobject analysis.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    KobjectAction, KobjectAnalysis, KobjectId, KobjectInfo, KobjectIssue, KobjectIssueType,
    KobjectRecommendation, KobjectState, KsetId, KsetInfo, KtypeId, KtypeInfo, LifecycleEventType,
    LifecycleTracker, RefCountAnalyzer, SysfsManager, UeventAction, UeventHandler,
};

/// Kobject Intelligence - comprehensive kernel object analysis and management
pub struct KobjectIntelligence {
    /// Registered kobjects
    kobjects: BTreeMap<KobjectId, KobjectInfo>,
    /// Registered ksets
    ksets: BTreeMap<KsetId, KsetInfo>,
    /// Registered ktypes
    ktypes: BTreeMap<KtypeId, KtypeInfo>,
    /// Reference count analyzer
    refcount_analyzer: RefCountAnalyzer,
    /// Sysfs manager
    sysfs_manager: SysfsManager,
    /// Uevent handler
    uevent_handler: UeventHandler,
    /// Lifecycle tracker
    lifecycle_tracker: LifecycleTracker,
    /// Next kobject ID
    next_kobject_id: AtomicU64,
    /// Next kset ID
    next_kset_id: AtomicU64,
    /// Next ktype ID
    next_ktype_id: AtomicU64,
    /// Total kobjects created
    total_created: AtomicU64,
    /// Total kobjects destroyed
    total_destroyed: AtomicU64,
}

impl KobjectIntelligence {
    /// Create new kobject intelligence
    pub fn new() -> Self {
        Self {
            kobjects: BTreeMap::new(),
            ksets: BTreeMap::new(),
            ktypes: BTreeMap::new(),
            refcount_analyzer: RefCountAnalyzer::new(),
            sysfs_manager: SysfsManager::new(),
            uevent_handler: UeventHandler::new(),
            lifecycle_tracker: LifecycleTracker::new(),
            next_kobject_id: AtomicU64::new(1),
            next_kset_id: AtomicU64::new(1),
            next_ktype_id: AtomicU64::new(1),
            total_created: AtomicU64::new(0),
            total_destroyed: AtomicU64::new(0),
        }
    }

    /// Allocate kobject ID
    pub fn allocate_kobject_id(&self) -> KobjectId {
        KobjectId::new(self.next_kobject_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Allocate kset ID
    pub fn allocate_kset_id(&self) -> KsetId {
        KsetId::new(self.next_kset_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Allocate ktype ID
    pub fn allocate_ktype_id(&self) -> KtypeId {
        KtypeId::new(self.next_ktype_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Register kobject
    pub fn register_kobject(&mut self, info: KobjectInfo, timestamp: u64) {
        let id = info.id;
        let path = info.path.clone();

        self.lifecycle_tracker.record_event(
            id,
            LifecycleEventType::Created,
            timestamp,
            info.name.clone(),
        );
        self.sysfs_manager.add_directory(path, id);
        self.kobjects.insert(id, info);
        self.total_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Unregister kobject
    pub fn unregister_kobject(&mut self, id: KobjectId, timestamp: u64) {
        if let Some(mut info) = self.kobjects.remove(&id) {
            info.state = KobjectState::Destroyed;
            self.lifecycle_tracker.record_event(
                id,
                LifecycleEventType::Released,
                timestamp,
                info.name.clone(),
            );
            self.sysfs_manager.remove_entry(&info.path);
            self.refcount_analyzer.clear_history(id);
            self.total_destroyed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get kobject
    pub fn get_kobject(&self, id: KobjectId) -> Option<&KobjectInfo> {
        self.kobjects.get(&id)
    }

    /// Increment refcount
    pub fn kobject_get(&mut self, id: KobjectId, caller: String, timestamp: u64) -> Option<u32> {
        if let Some(info) = self.kobjects.get_mut(&id) {
            info.refcount += 1;
            info.last_access = timestamp;
            self.refcount_analyzer
                .record_get(id, info.refcount, caller, timestamp);
            return Some(info.refcount);
        }
        None
    }

    /// Decrement refcount
    pub fn kobject_put(&mut self, id: KobjectId, caller: String, timestamp: u64) -> Option<u32> {
        if let Some(info) = self.kobjects.get_mut(&id) {
            if info.refcount == 0 {
                self.refcount_analyzer
                    .record_put(id, u32::MAX, caller, timestamp);
                return None;
            }
            info.refcount -= 1;
            info.last_access = timestamp;
            self.refcount_analyzer
                .record_put(id, info.refcount, caller, timestamp);

            if info.refcount == 0 {
                self.unregister_kobject(id, timestamp);
            }

            return Some(info.refcount);
        }
        None
    }

    /// Register kset
    pub fn register_kset(&mut self, info: KsetInfo) {
        self.ksets.insert(info.id, info);
    }

    /// Register ktype
    pub fn register_ktype(&mut self, info: KtypeInfo) {
        self.ktypes.insert(info.id, info);
    }

    /// Send uevent
    pub fn send_uevent(
        &mut self,
        id: KobjectId,
        action: UeventAction,
        timestamp: u64,
    ) -> Option<u64> {
        let info = self.kobjects.get(&id)?;
        if info.uevent_suppressed {
            return None;
        }

        let seqnum = self.uevent_handler.queue_uevent(
            action,
            info.path.clone(),
            String::from("subsystem"),
            id,
            timestamp,
        );

        Some(seqnum)
    }

    /// Analyze kobject
    pub fn analyze_kobject(&mut self, id: KobjectId, current_time: u64) -> Option<KobjectAnalysis> {
        let info = self.kobjects.get(&id)?;
        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check for potential leaks
        let gets = self.refcount_analyzer.total_gets();
        let puts = self.refcount_analyzer.total_puts();
        if gets > puts + 10 {
            health_score -= 20.0;
            issues.push(KobjectIssue {
                issue_type: KobjectIssueType::RefLeak,
                severity: 7,
                description: String::from("Potential reference count leak detected"),
            });
            recommendations.push(KobjectRecommendation {
                action: KobjectAction::FixRefcount,
                expected_improvement: 20.0,
                reason: String::from("Balance get/put operations"),
            });
        }

        // Check for underflows
        if self.refcount_analyzer.underflow_count() > 0 {
            health_score -= 30.0;
            issues.push(KobjectIssue {
                issue_type: KobjectIssueType::RefUnderflow,
                severity: 9,
                description: String::from("Reference count underflow detected"),
            });
        }

        // Check for missing release
        if let Some(ktype_id) = info.ktype {
            if let Some(ktype) = self.ktypes.get(&ktype_id) {
                if !ktype.has_release {
                    health_score -= 15.0;
                    issues.push(KobjectIssue {
                        issue_type: KobjectIssueType::MissingRelease,
                        severity: 5,
                        description: String::from("Ktype has no release function"),
                    });
                    recommendations.push(KobjectRecommendation {
                        action: KobjectAction::AddRelease,
                        expected_improvement: 15.0,
                        reason: String::from("Add proper release function"),
                    });
                }
            }
        }

        // Check for long-lived objects
        let age = current_time.saturating_sub(info.created_at);
        if age > 3600_000_000_000 && info.refcount > 1 {
            health_score -= 10.0;
            issues.push(KobjectIssue {
                issue_type: KobjectIssueType::LongLived,
                severity: 3,
                description: String::from("Long-lived object with extra references"),
            });
        }

        health_score = health_score.max(0.0);

        Some(KobjectAnalysis {
            kobject_id: id,
            health_score,
            issues,
            recommendations,
        })
    }

    /// Get refcount analyzer
    pub fn refcount_analyzer(&self) -> &RefCountAnalyzer {
        &self.refcount_analyzer
    }

    /// Get refcount analyzer mutably
    pub fn refcount_analyzer_mut(&mut self) -> &mut RefCountAnalyzer {
        &mut self.refcount_analyzer
    }

    /// Get sysfs manager
    pub fn sysfs_manager(&self) -> &SysfsManager {
        &self.sysfs_manager
    }

    /// Get sysfs manager mutably
    pub fn sysfs_manager_mut(&mut self) -> &mut SysfsManager {
        &mut self.sysfs_manager
    }

    /// Get uevent handler
    pub fn uevent_handler(&self) -> &UeventHandler {
        &self.uevent_handler
    }

    /// Get uevent handler mutably
    pub fn uevent_handler_mut(&mut self) -> &mut UeventHandler {
        &mut self.uevent_handler
    }

    /// Get lifecycle tracker
    pub fn lifecycle_tracker(&self) -> &LifecycleTracker {
        &self.lifecycle_tracker
    }

    /// Get total created
    pub fn total_created(&self) -> u64 {
        self.total_created.load(Ordering::Relaxed)
    }

    /// Get total destroyed
    pub fn total_destroyed(&self) -> u64 {
        self.total_destroyed.load(Ordering::Relaxed)
    }

    /// Get active kobject count
    pub fn active_count(&self) -> usize {
        self.kobjects.len()
    }
}

impl Default for KobjectIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
