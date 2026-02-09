//! Tags and Labels
//!
//! Key-value metadata for annotating entities.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// TAG
// ============================================================================

/// A key-value tag
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    /// Tag key
    pub key: String,
    /// Tag value
    pub value: String,
}

impl Tag {
    /// Create new tag
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Create from static strings (no allocation)
    #[inline]
    pub fn from_static(key: &'static str, value: &'static str) -> Self {
        Self {
            key: String::from(key),
            value: String::from(value),
        }
    }
}

impl core::fmt::Display for Tag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

// ============================================================================
// TAGS
// ============================================================================

/// Collection of tags
#[derive(Debug, Clone, Default)]
pub struct Tags(pub Vec<Tag>);

impl Tags {
    /// Create empty tags
    #[inline(always)]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Create with capacity
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Add tag
    #[inline(always)]
    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.push(Tag::new(key, value));
    }

    /// Add tag, replacing if key exists
    #[inline]
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        if let Some(tag) = self.0.iter_mut().find(|t| t.key == key) {
            tag.value = value.into();
        } else {
            self.0.push(Tag {
                key,
                value: value.into(),
            });
        }
    }

    /// Remove tag by key
    #[inline]
    pub fn remove(&mut self, key: &str) -> Option<Tag> {
        if let Some(idx) = self.0.iter().position(|t| t.key == key) {
            Some(self.0.remove(idx))
        } else {
            None
        }
    }

    /// Get tag value
    #[inline]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|t| t.key == key)
            .map(|t| t.value.as_str())
    }

    /// Has tag
    #[inline(always)]
    pub fn has(&self, key: &str) -> bool {
        self.0.iter().any(|t| t.key == key)
    }

    /// Number of tags
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over tags
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &Tag> {
        self.0.iter()
    }

    /// Get keys
    #[inline(always)]
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|t| t.key.as_str())
    }

    /// Get values
    #[inline(always)]
    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|t| t.value.as_str())
    }

    /// Merge with another tags collection
    #[inline]
    pub fn merge(&mut self, other: Tags) {
        for tag in other.0 {
            self.set(tag.key, tag.value);
        }
    }

    /// Filter tags by predicate
    #[inline(always)]
    pub fn filter<F>(&self, predicate: F) -> Tags
    where
        F: Fn(&Tag) -> bool,
    {
        Tags(self.0.iter().filter(|t| predicate(t)).cloned().collect())
    }

    /// Clear all tags
    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl FromIterator<Tag> for Tags {
    fn from_iter<I: IntoIterator<Item = Tag>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a> IntoIterator for &'a Tags {
    type Item = &'a Tag;
    type IntoIter = core::slice::Iter<'a, Tag>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// ============================================================================
// LABEL
// ============================================================================

/// A label (simple string identifier)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(pub String);

impl Label {
    /// Create new label
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get value
    #[inline(always)]
    pub fn value(&self) -> &str {
        &self.0
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl core::fmt::Display for Label {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Collection of labels
#[derive(Debug, Clone, Default)]
pub struct Labels(pub Vec<Label>);

impl Labels {
    /// Create empty labels
    #[inline(always)]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Add label
    #[inline]
    pub fn add(&mut self, label: impl Into<String>) {
        let label = Label::new(label);
        if !self.has(&label.0) {
            self.0.push(label);
        }
    }

    /// Has label
    #[inline(always)]
    pub fn has(&self, label: &str) -> bool {
        self.0.iter().any(|l| l.0 == label)
    }

    /// Remove label
    #[inline]
    pub fn remove(&mut self, label: &str) -> bool {
        if let Some(idx) = self.0.iter().position(|l| l.0 == label) {
            self.0.remove(idx);
            true
        } else {
            false
        }
    }

    /// Number of labels
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.0.iter()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag() {
        let tag = Tag::new("env", "prod");
        assert_eq!(tag.key, "env");
        assert_eq!(tag.value, "prod");
    }

    #[test]
    fn test_tags() {
        let mut tags = Tags::new();
        tags.add("env", "prod");
        tags.add("team", "kernel");
        assert_eq!(tags.get("env"), Some("prod"));
        assert!(tags.has("team"));
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_tags_set() {
        let mut tags = Tags::new();
        tags.set("key", "value1");
        tags.set("key", "value2");
        assert_eq!(tags.get("key"), Some("value2"));
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_labels() {
        let mut labels = Labels::new();
        labels.add("important");
        labels.add("urgent");
        labels.add("important"); // Duplicate, should be ignored
        assert_eq!(labels.len(), 2);
        assert!(labels.has("important"));
    }
}
