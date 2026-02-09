//! Generic Wrappers
//!
//! Wrapper types for adding metadata to values.

#![allow(dead_code)]

use super::temporal::{Duration, Timestamp};
use super::version::Version;

// ============================================================================
// VERSIONED
// ============================================================================

/// Versioned wrapper for any type
#[derive(Debug, Clone)]
pub struct Versioned<T> {
    /// The data
    pub data: T,
    /// Version
    pub version: Version,
    /// Created timestamp
    pub created: Timestamp,
    /// Last modified timestamp
    pub modified: Timestamp,
}

impl<T> Versioned<T> {
    /// Create new versioned data
    pub fn new(data: T) -> Self {
        let now = Timestamp::now();
        Self {
            data,
            version: Version::INITIAL,
            created: now,
            modified: now,
        }
    }

    /// Create with specific version
    #[inline]
    pub fn with_version(data: T, version: Version) -> Self {
        let now = Timestamp::now();
        Self {
            data,
            version,
            created: now,
            modified: now,
        }
    }

    /// Update data (bumps patch version)
    #[inline]
    pub fn update(&mut self, data: T) {
        self.data = data;
        self.modified = Timestamp::now();
        self.version = self.version.bump_patch();
    }

    /// Get reference to data
    #[inline(always)]
    pub fn get(&self) -> &T {
        &self.data
    }

    /// Get mutable reference to data (updates modified time)
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.modified = Timestamp::now();
        &mut self.data
    }

    /// Into inner data
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.data
    }
}

// ============================================================================
// TIMESTAMPED
// ============================================================================

/// Timestamped wrapper
#[derive(Debug, Clone)]
pub struct Timestamped<T> {
    /// The data
    pub data: T,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl<T> Timestamped<T> {
    /// Create new timestamped data
    pub fn new(data: T) -> Self {
        Self {
            data,
            timestamp: Timestamp::now(),
        }
    }

    /// Create with specific timestamp
    #[inline(always)]
    pub fn at(data: T, timestamp: Timestamp) -> Self {
        Self { data, timestamp }
    }

    /// Get reference to data
    #[inline(always)]
    pub fn get(&self) -> &T {
        &self.data
    }

    /// Into inner data
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.data
    }

    /// Age since creation
    #[inline(always)]
    pub fn age(&self, now: Timestamp) -> Duration {
        now.elapsed_since(self.timestamp)
    }
}

// ============================================================================
// EXPIRING
// ============================================================================

/// Expiring wrapper (data with TTL)
#[derive(Debug, Clone)]
pub struct Expiring<T> {
    /// The data
    pub data: T,
    /// Created timestamp
    pub created: Timestamp,
    /// Time to live
    pub ttl: Duration,
}

impl<T> Expiring<T> {
    /// Create new expiring data
    pub fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            created: Timestamp::now(),
            ttl,
        }
    }

    /// Create with specific creation time
    #[inline(always)]
    pub fn new_at(data: T, created: Timestamp, ttl: Duration) -> Self {
        Self { data, created, ttl }
    }

    /// Check if expired
    #[inline(always)]
    pub fn is_expired(&self, now: Timestamp) -> bool {
        now.elapsed_since(self.created).0 > self.ttl.0
    }

    /// Check if still valid
    #[inline(always)]
    pub fn is_valid(&self, now: Timestamp) -> bool {
        !self.is_expired(now)
    }

    /// Remaining time
    #[inline]
    pub fn remaining(&self, now: Timestamp) -> Duration {
        let elapsed = now.elapsed_since(self.created);
        if elapsed.0 >= self.ttl.0 {
            Duration::ZERO
        } else {
            Duration::from_nanos(self.ttl.0 - elapsed.0)
        }
    }

    /// Extend TTL
    #[inline(always)]
    pub fn extend(&mut self, additional: Duration) {
        self.ttl = self.ttl.saturating_add(additional);
    }

    /// Reset TTL (restart countdown)
    #[inline(always)]
    pub fn reset(&mut self) {
        self.created = Timestamp::now();
    }

    /// Get reference to data (only if not expired)
    #[inline]
    pub fn get(&self, now: Timestamp) -> Option<&T> {
        if self.is_valid(now) {
            Some(&self.data)
        } else {
            None
        }
    }

    /// Into inner data (regardless of expiration)
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.data
    }
}

// ============================================================================
// COUNTED
// ============================================================================

/// Counted wrapper (tracks access count)
#[derive(Debug, Clone)]
pub struct Counted<T> {
    /// The data
    pub data: T,
    /// Access count
    pub count: u64,
    /// Last access timestamp
    pub last_access: Timestamp,
}

impl<T> Counted<T> {
    /// Create new counted data
    pub fn new(data: T) -> Self {
        Self {
            data,
            count: 0,
            last_access: Timestamp::now(),
        }
    }

    /// Get reference to data (increments count)
    #[inline]
    pub fn get(&mut self) -> &T {
        self.count += 1;
        self.last_access = Timestamp::now();
        &self.data
    }

    /// Get without counting
    #[inline(always)]
    pub fn peek(&self) -> &T {
        &self.data
    }

    /// Get access count
    #[inline(always)]
    pub fn access_count(&self) -> u64 {
        self.count
    }

    /// Reset count
    #[inline(always)]
    pub fn reset_count(&mut self) {
        self.count = 0;
    }

    /// Into inner data
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.data
    }
}

// ============================================================================
// OPTIONAL
// ============================================================================

/// Optional with metadata
#[derive(Debug, Clone)]
pub struct OptionalWith<T, M> {
    /// The optional data
    pub data: Option<T>,
    /// Metadata (always present)
    pub meta: M,
}

impl<T, M> OptionalWith<T, M> {
    /// Create with data
    #[inline]
    pub fn some(data: T, meta: M) -> Self {
        Self {
            data: Some(data),
            meta,
        }
    }

    /// Create without data
    #[inline(always)]
    pub fn none(meta: M) -> Self {
        Self { data: None, meta }
    }

    /// Is some
    #[inline(always)]
    pub fn is_some(&self) -> bool {
        self.data.is_some()
    }

    /// Is none
    #[inline(always)]
    pub fn is_none(&self) -> bool {
        self.data.is_none()
    }

    /// Get data reference
    #[inline(always)]
    pub fn get(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Get metadata reference
    #[inline(always)]
    pub fn metadata(&self) -> &M {
        &self.meta
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versioned() {
        let mut v = Versioned::new(42);
        assert_eq!(v.version, Version::INITIAL);
        v.update(43);
        assert_eq!(v.version.patch, 1);
        assert_eq!(*v.get(), 43);
    }

    #[test]
    fn test_timestamped() {
        let ts = Timestamped::new(100);
        assert_eq!(*ts.get(), 100);
    }

    #[test]
    fn test_expiring() {
        let exp = Expiring::new(42, Duration::from_secs(60));
        let now = Timestamp::now();
        assert!(exp.is_valid(now));
        assert!(exp.get(now).is_some());
    }

    #[test]
    fn test_counted() {
        let mut c = Counted::new("data");
        assert_eq!(c.access_count(), 0);
        let _ = c.get();
        let _ = c.get();
        assert_eq!(c.access_count(), 2);
    }
}
