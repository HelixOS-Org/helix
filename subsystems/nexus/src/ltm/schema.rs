//! # Memory Schema
//!
//! Defines and manages memory schemas for structured storage.
//! Implements schema evolution and validation.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// SCHEMA TYPES
// ============================================================================

/// Memory schema
#[derive(Debug, Clone)]
pub struct MemorySchema {
    /// Schema ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Version
    pub version: u32,
    /// Fields
    pub fields: Vec<SchemaField>,
    /// Indexes
    pub indexes: Vec<SchemaIndex>,
    /// Constraints
    pub constraints: Vec<SchemaConstraint>,
    /// Created
    pub created: Timestamp,
}

/// Schema field
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Field ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub field_type: FieldType,
    /// Required
    pub required: bool,
    /// Default value
    pub default: Option<FieldValue>,
}

/// Field type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Bool,
    Int,
    Float,
    String,
    Bytes,
    List,
    Map,
    Reference,
    Timestamp,
}

/// Field value
#[derive(Debug, Clone)]
pub enum FieldValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<FieldValue>),
    Map(BTreeMap<String, FieldValue>),
    Reference(u64),
    Timestamp(Timestamp),
    Null,
}

/// Schema index
#[derive(Debug, Clone)]
pub struct SchemaIndex {
    /// Index ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Fields
    pub fields: Vec<u64>,
    /// Unique
    pub unique: bool,
}

/// Schema constraint
#[derive(Debug, Clone)]
pub struct SchemaConstraint {
    /// Constraint ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub constraint_type: ConstraintType,
    /// Fields
    pub fields: Vec<u64>,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    Unique,
    NotNull,
    Check,
    ForeignKey,
    Range,
}

/// Memory record
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    /// Record ID
    pub id: u64,
    /// Schema ID
    pub schema: u64,
    /// Values
    pub values: BTreeMap<u64, FieldValue>,
    /// Created
    pub created: Timestamp,
    /// Modified
    pub modified: Timestamp,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Valid
    pub valid: bool,
    /// Errors
    pub errors: Vec<ValidationError>,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Field ID
    pub field: u64,
    /// Error message
    pub message: String,
}

// ============================================================================
// SCHEMA MANAGER
// ============================================================================

/// Schema manager
pub struct SchemaManager {
    /// Schemas
    schemas: BTreeMap<u64, MemorySchema>,
    /// Schema versions
    versions: BTreeMap<String, Vec<u64>>, // name -> [schema_ids]
    /// Records
    records: BTreeMap<u64, MemoryRecord>,
    /// Indexes
    index_data: BTreeMap<u64, BTreeMap<String, Vec<u64>>>, // index_id -> key -> record_ids
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: SchemaConfig,
    /// Statistics
    stats: SchemaStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SchemaConfig {
    /// Maximum fields
    pub max_fields: usize,
    /// Maximum records per schema
    pub max_records: usize,
    /// Enable validation
    pub validate: bool,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            max_fields: 100,
            max_records: 10000,
            validate: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct SchemaStats {
    /// Schemas created
    pub schemas_created: u64,
    /// Records inserted
    pub records_inserted: u64,
    /// Validations performed
    pub validations: u64,
    /// Validation failures
    pub validation_failures: u64,
}

impl SchemaManager {
    /// Create new manager
    pub fn new(config: SchemaConfig) -> Self {
        Self {
            schemas: BTreeMap::new(),
            versions: BTreeMap::new(),
            records: BTreeMap::new(),
            index_data: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: SchemaStats::default(),
        }
    }

    /// Create schema
    pub fn create_schema(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let version = self
            .versions
            .get(name)
            .map(|v| v.len() as u32 + 1)
            .unwrap_or(1);

        let schema = MemorySchema {
            id,
            name: name.into(),
            version,
            fields: Vec::new(),
            indexes: Vec::new(),
            constraints: Vec::new(),
            created: Timestamp::now(),
        };

        self.schemas.insert(id, schema);
        self.versions
            .entry(name.into())
            .or_insert_with(Vec::new)
            .push(id);

        self.stats.schemas_created += 1;

        id
    }

    /// Add field
    pub fn add_field(
        &mut self,
        schema_id: u64,
        name: &str,
        field_type: FieldType,
        required: bool,
        default: Option<FieldValue>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let field = SchemaField {
            id,
            name: name.into(),
            field_type,
            required,
            default,
        };

        if let Some(schema) = self.schemas.get_mut(&schema_id) {
            if schema.fields.len() < self.config.max_fields {
                schema.fields.push(field);
            }
        }

        id
    }

    /// Add index
    pub fn add_index(&mut self, schema_id: u64, name: &str, fields: Vec<u64>, unique: bool) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let index = SchemaIndex {
            id,
            name: name.into(),
            fields,
            unique,
        };

        if let Some(schema) = self.schemas.get_mut(&schema_id) {
            schema.indexes.push(index);
        }

        self.index_data.insert(id, BTreeMap::new());

        id
    }

    /// Add constraint
    pub fn add_constraint(
        &mut self,
        schema_id: u64,
        name: &str,
        constraint_type: ConstraintType,
        fields: Vec<u64>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let constraint = SchemaConstraint {
            id,
            name: name.into(),
            constraint_type,
            fields,
        };

        if let Some(schema) = self.schemas.get_mut(&schema_id) {
            schema.constraints.push(constraint);
        }

        id
    }

    /// Insert record
    pub fn insert(&mut self, schema_id: u64, values: BTreeMap<u64, FieldValue>) -> Option<u64> {
        let schema = self.schemas.get(&schema_id)?.clone();

        // Validate
        if self.config.validate {
            let result = self.validate(&schema, &values);
            self.stats.validations += 1;

            if !result.valid {
                self.stats.validation_failures += 1;
                return None;
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let record = MemoryRecord {
            id,
            schema: schema_id,
            values: values.clone(),
            created: now,
            modified: now,
        };

        // Update indexes
        for index in &schema.indexes {
            let key = self.build_index_key(&index.fields, &values);
            self.index_data
                .entry(index.id)
                .or_insert_with(BTreeMap::new)
                .entry(key)
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.records.insert(id, record);
        self.stats.records_inserted += 1;

        Some(id)
    }

    fn build_index_key(&self, fields: &[u64], values: &BTreeMap<u64, FieldValue>) -> String {
        fields
            .iter()
            .filter_map(|f| values.get(f))
            .map(|v| self.value_to_string(v))
            .collect::<Vec<_>>()
            .join("|")
    }

    fn value_to_string(&self, value: &FieldValue) -> String {
        match value {
            FieldValue::Bool(b) => b.to_string(),
            FieldValue::Int(i) => i.to_string(),
            FieldValue::Float(f) => format!("{:.6}", f),
            FieldValue::String(s) => s.clone(),
            FieldValue::Reference(r) => r.to_string(),
            FieldValue::Timestamp(t) => t.0.to_string(),
            _ => "".into(),
        }
    }

    /// Validate record
    pub fn validate(
        &self,
        schema: &MemorySchema,
        values: &BTreeMap<u64, FieldValue>,
    ) -> ValidationResult {
        let mut errors = Vec::new();

        // Check required fields
        for field in &schema.fields {
            if field.required && !values.contains_key(&field.id) {
                errors.push(ValidationError {
                    field: field.id,
                    message: format!("Required field '{}' missing", field.name),
                });
            }
        }

        // Check types
        for (field_id, value) in values {
            if let Some(field) = schema.fields.iter().find(|f| f.id == *field_id) {
                if !self.type_matches(field.field_type, value) {
                    errors.push(ValidationError {
                        field: *field_id,
                        message: format!("Type mismatch for field '{}'", field.name),
                    });
                }
            }
        }

        // Check constraints
        for constraint in &schema.constraints {
            if let Some(error) = self.check_constraint(constraint, values) {
                errors.push(error);
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
        }
    }

    fn type_matches(&self, expected: FieldType, value: &FieldValue) -> bool {
        matches!(
            (expected, value),
            (FieldType::Bool, FieldValue::Bool(_))
                | (FieldType::Int, FieldValue::Int(_))
                | (FieldType::Float, FieldValue::Float(_))
                | (FieldType::String, FieldValue::String(_))
                | (FieldType::Bytes, FieldValue::Bytes(_))
                | (FieldType::List, FieldValue::List(_))
                | (FieldType::Map, FieldValue::Map(_))
                | (FieldType::Reference, FieldValue::Reference(_))
                | (FieldType::Timestamp, FieldValue::Timestamp(_))
                | (_, FieldValue::Null)
        )
    }

    fn check_constraint(
        &self,
        constraint: &SchemaConstraint,
        values: &BTreeMap<u64, FieldValue>,
    ) -> Option<ValidationError> {
        match constraint.constraint_type {
            ConstraintType::NotNull => {
                for field_id in &constraint.fields {
                    if let Some(FieldValue::Null) = values.get(field_id) {
                        return Some(ValidationError {
                            field: *field_id,
                            message: "Null value not allowed".into(),
                        });
                    }
                }
            },
            ConstraintType::Unique => {
                // Would check against existing records
            },
            _ => {},
        }
        None
    }

    /// Update record
    pub fn update(&mut self, record_id: u64, values: BTreeMap<u64, FieldValue>) -> bool {
        if let Some(record) = self.records.get_mut(&record_id) {
            if self.config.validate {
                if let Some(schema) = self.schemas.get(&record.schema) {
                    let result = self.validate(schema, &values);
                    if !result.valid {
                        return false;
                    }
                }
            }

            record.values = values;
            record.modified = Timestamp::now();
            return true;
        }
        false
    }

    /// Query by index
    pub fn query_by_index(&self, index_id: u64, key: &str) -> Vec<&MemoryRecord> {
        self.index_data
            .get(&index_id)
            .and_then(|idx| idx.get(key))
            .map(|ids| ids.iter().filter_map(|id| self.records.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get record
    pub fn get(&self, id: u64) -> Option<&MemoryRecord> {
        self.records.get(&id)
    }

    /// Get schema
    pub fn get_schema(&self, id: u64) -> Option<&MemorySchema> {
        self.schemas.get(&id)
    }

    /// Get schema by name
    pub fn get_schema_by_name(&self, name: &str) -> Option<&MemorySchema> {
        self.versions
            .get(name)
            .and_then(|ids| ids.last())
            .and_then(|id| self.schemas.get(id))
    }

    /// List records by schema
    pub fn list_by_schema(&self, schema_id: u64) -> Vec<&MemoryRecord> {
        self.records
            .values()
            .filter(|r| r.schema == schema_id)
            .collect()
    }

    /// Migrate schema
    pub fn migrate(&mut self, old_id: u64, migration: impl Fn(&FieldValue) -> FieldValue) {
        let record_ids: Vec<u64> = self
            .records
            .values()
            .filter(|r| r.schema == old_id)
            .map(|r| r.id)
            .collect();

        for id in record_ids {
            if let Some(record) = self.records.get_mut(&id) {
                for value in record.values.values_mut() {
                    *value = migration(value);
                }
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &SchemaStats {
        &self.stats
    }
}

impl Default for SchemaManager {
    fn default() -> Self {
        Self::new(SchemaConfig::default())
    }
}

// ============================================================================
// BUILDER
// ============================================================================

/// Schema builder
pub struct SchemaBuilder<'a> {
    manager: &'a mut SchemaManager,
    schema_id: u64,
}

impl<'a> SchemaBuilder<'a> {
    /// Create new builder
    pub fn new(manager: &'a mut SchemaManager, name: &str) -> Self {
        let schema_id = manager.create_schema(name);
        Self { manager, schema_id }
    }

    /// Add field
    pub fn field(self, name: &str, field_type: FieldType, required: bool) -> Self {
        self.manager
            .add_field(self.schema_id, name, field_type, required, None);
        self
    }

    /// Add field with default
    pub fn field_default(self, name: &str, field_type: FieldType, default: FieldValue) -> Self {
        self.manager
            .add_field(self.schema_id, name, field_type, false, Some(default));
        self
    }

    /// Add index
    pub fn index(self, name: &str, fields: Vec<u64>, unique: bool) -> Self {
        self.manager.add_index(self.schema_id, name, fields, unique);
        self
    }

    /// Build and return schema ID
    pub fn build(self) -> u64 {
        self.schema_id
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_schema() {
        let mut manager = SchemaManager::default();

        let id = manager.create_schema("users");
        assert!(manager.get_schema(id).is_some());
    }

    #[test]
    fn test_add_field() {
        let mut manager = SchemaManager::default();

        let schema = manager.create_schema("test");
        manager.add_field(schema, "name", FieldType::String, true, None);

        let s = manager.get_schema(schema).unwrap();
        assert_eq!(s.fields.len(), 1);
    }

    #[test]
    fn test_insert() {
        let mut manager = SchemaManager::default();

        let schema = manager.create_schema("test");
        let name_field = manager.add_field(schema, "name", FieldType::String, true, None);

        let mut values = BTreeMap::new();
        values.insert(name_field, FieldValue::String("test".into()));

        let id = manager.insert(schema, values);
        assert!(id.is_some());
    }

    #[test]
    fn test_validation_required() {
        let mut manager = SchemaManager::default();

        let schema = manager.create_schema("test");
        manager.add_field(schema, "required_field", FieldType::String, true, None);

        let values = BTreeMap::new(); // Missing required field

        let id = manager.insert(schema, values);
        assert!(id.is_none());
    }

    #[test]
    fn test_validation_type() {
        let mut manager = SchemaManager::default();

        let schema = manager.create_schema("test");
        let int_field = manager.add_field(schema, "count", FieldType::Int, true, None);

        let mut values = BTreeMap::new();
        values.insert(int_field, FieldValue::String("not an int".into())); // Wrong type

        let id = manager.insert(schema, values);
        assert!(id.is_none());
    }

    #[test]
    fn test_get_by_name() {
        let mut manager = SchemaManager::default();

        manager.create_schema("users");

        let schema = manager.get_schema_by_name("users");
        assert!(schema.is_some());
    }

    #[test]
    fn test_builder() {
        let mut manager = SchemaManager::default();

        let schema = SchemaBuilder::new(&mut manager, "products")
            .field("name", FieldType::String, true)
            .field("price", FieldType::Float, true)
            .field_default("stock", FieldType::Int, FieldValue::Int(0))
            .build();

        let s = manager.get_schema(schema).unwrap();
        assert_eq!(s.fields.len(), 3);
    }
}
