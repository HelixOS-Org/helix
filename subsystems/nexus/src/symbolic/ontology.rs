//! # Ontology Engine for NEXUS
//!
//! Year 2 "COGNITION" - Revolutionary kernel-level ontology management
//! system that provides semantic understanding of kernel concepts,
//! hierarchical classification, and reasoning over class structures.
//!
//! ## Features
//!
//! - OWL-like class hierarchies
//! - Property definitions (object, data, annotation)
//! - Individual instances
//! - Subsumption reasoning
//! - Instance classification
//! - SWRL-like rules
//! - Kernel domain ontology (processes, memory, devices)

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum classes in ontology
const MAX_CLASSES: usize = 10_000;

/// Maximum properties
const MAX_PROPERTIES: usize = 5_000;

/// Maximum individuals
const MAX_INDIVIDUALS: usize = 100_000;

/// Maximum inheritance depth
const MAX_INHERITANCE_DEPTH: usize = 50;

// ============================================================================
// CORE IDENTIFIERS
// ============================================================================

/// Class identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassId(pub u32);

/// Property identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropertyId(pub u32);

/// Individual identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndividualId(pub u32);

/// Special class IDs
impl ClassId {
    /// owl:Thing - the top class
    pub const THING: ClassId = ClassId(0);
    /// owl:Nothing - the bottom class
    pub const NOTHING: ClassId = ClassId(1);
}

// ============================================================================
// DATA VALUES
// ============================================================================

/// Data type for literals
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// xsd:string
    String,
    /// xsd:integer
    Integer,
    /// xsd:boolean
    Boolean,
    /// xsd:float
    Float,
    /// xsd:dateTime
    DateTime,
    /// Custom type
    Custom(u32),
}

/// A data value (literal)
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// Float value
    Float(f64),
    /// DateTime as timestamp
    DateTime(u64),
    /// Null/undefined
    Null,
}

impl DataValue {
    /// Get data type
    pub fn data_type(&self) -> DataType {
        match self {
            DataValue::String(_) => DataType::String,
            DataValue::Integer(_) => DataType::Integer,
            DataValue::Boolean(_) => DataType::Boolean,
            DataValue::Float(_) => DataType::Float,
            DataValue::DateTime(_) => DataType::DateTime,
            DataValue::Null => DataType::String,
        }
    }
}

// ============================================================================
// CLASSES
// ============================================================================

/// Class expression (for complex class definitions)
#[derive(Debug, Clone)]
pub enum ClassExpression {
    /// Named class
    Named(ClassId),
    /// Intersection of classes (owl:intersectionOf)
    Intersection(Vec<ClassExpression>),
    /// Union of classes (owl:unionOf)
    Union(Vec<ClassExpression>),
    /// Complement of class (owl:complementOf)
    Complement(Box<ClassExpression>),
    /// Universal restriction (owl:allValuesFrom)
    AllValuesFrom(PropertyId, Box<ClassExpression>),
    /// Existential restriction (owl:someValuesFrom)
    SomeValuesFrom(PropertyId, Box<ClassExpression>),
    /// Has value restriction (owl:hasValue)
    HasValue(PropertyId, IndividualId),
    /// Cardinality restriction (owl:cardinality)
    ExactCardinality(PropertyId, usize),
    /// Min cardinality (owl:minCardinality)
    MinCardinality(PropertyId, usize),
    /// Max cardinality (owl:maxCardinality)
    MaxCardinality(PropertyId, usize),
    /// One of (owl:oneOf)
    OneOf(Vec<IndividualId>),
    /// Has self (owl:hasSelf)
    HasSelf(PropertyId),
}

impl ClassExpression {
    /// Is this a named class?
    pub fn is_named(&self) -> bool {
        matches!(self, ClassExpression::Named(_))
    }

    /// Get the named class ID if this is a named class
    pub fn as_named(&self) -> Option<ClassId> {
        match self {
            ClassExpression::Named(id) => Some(*id),
            _ => None,
        }
    }
}

/// A class definition
#[derive(Debug, Clone)]
pub struct OntologyClass {
    /// Class ID
    pub id: ClassId,
    /// IRI/name of the class
    pub iri: String,
    /// Label (human-readable name)
    pub label: Option<String>,
    /// Comment/description
    pub comment: Option<String>,
    /// Direct superclasses
    pub superclasses: Vec<ClassId>,
    /// Equivalent class expressions
    pub equivalent: Vec<ClassExpression>,
    /// Disjoint classes
    pub disjoint_with: Vec<ClassId>,
    /// Is deprecated?
    pub deprecated: bool,
    /// Custom annotations
    pub annotations: BTreeMap<String, DataValue>,
}

impl OntologyClass {
    /// Create a new class
    pub fn new(id: ClassId, iri: String) -> Self {
        Self {
            id,
            iri,
            label: None,
            comment: None,
            superclasses: Vec::new(),
            equivalent: Vec::new(),
            disjoint_with: Vec::new(),
            deprecated: false,
            annotations: BTreeMap::new(),
        }
    }

    /// Add a superclass
    pub fn add_superclass(&mut self, superclass: ClassId) {
        if !self.superclasses.contains(&superclass) {
            self.superclasses.push(superclass);
        }
    }

    /// Add an equivalent class expression
    pub fn add_equivalent(&mut self, expr: ClassExpression) {
        self.equivalent.push(expr);
    }

    /// Mark as disjoint with another class
    pub fn add_disjoint(&mut self, other: ClassId) {
        if !self.disjoint_with.contains(&other) {
            self.disjoint_with.push(other);
        }
    }
}

// ============================================================================
// PROPERTIES
// ============================================================================

/// Property type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    /// Object property (links individuals)
    Object,
    /// Data property (links individual to data value)
    Data,
    /// Annotation property
    Annotation,
}

/// Property characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyCharacteristic {
    /// Functional property (at most one value)
    Functional,
    /// Inverse functional
    InverseFunctional,
    /// Symmetric property
    Symmetric,
    /// Asymmetric property
    Asymmetric,
    /// Transitive property
    Transitive,
    /// Reflexive property
    Reflexive,
    /// Irreflexive property
    Irreflexive,
}

/// A property definition
#[derive(Debug, Clone)]
pub struct OntologyProperty {
    /// Property ID
    pub id: PropertyId,
    /// IRI/name
    pub iri: String,
    /// Property type
    pub property_type: PropertyType,
    /// Label
    pub label: Option<String>,
    /// Comment
    pub comment: Option<String>,
    /// Domain classes
    pub domain: Vec<ClassId>,
    /// Range (classes for object, data types for data)
    pub range_classes: Vec<ClassId>,
    /// Range data type (for data properties)
    pub range_datatype: Option<DataType>,
    /// Super properties
    pub super_properties: Vec<PropertyId>,
    /// Inverse property
    pub inverse_of: Option<PropertyId>,
    /// Property characteristics
    pub characteristics: Vec<PropertyCharacteristic>,
    /// Is deprecated?
    pub deprecated: bool,
}

impl OntologyProperty {
    /// Create a new object property
    pub fn new_object(id: PropertyId, iri: String) -> Self {
        Self {
            id,
            iri,
            property_type: PropertyType::Object,
            label: None,
            comment: None,
            domain: Vec::new(),
            range_classes: Vec::new(),
            range_datatype: None,
            super_properties: Vec::new(),
            inverse_of: None,
            characteristics: Vec::new(),
            deprecated: false,
        }
    }

    /// Create a new data property
    pub fn new_data(id: PropertyId, iri: String, datatype: DataType) -> Self {
        Self {
            id,
            iri,
            property_type: PropertyType::Data,
            label: None,
            comment: None,
            domain: Vec::new(),
            range_classes: Vec::new(),
            range_datatype: Some(datatype),
            super_properties: Vec::new(),
            inverse_of: None,
            characteristics: Vec::new(),
            deprecated: false,
        }
    }

    /// Add a characteristic
    pub fn add_characteristic(&mut self, char: PropertyCharacteristic) {
        if !self.characteristics.contains(&char) {
            self.characteristics.push(char);
        }
    }

    /// Check if functional
    pub fn is_functional(&self) -> bool {
        self.characteristics
            .contains(&PropertyCharacteristic::Functional)
    }

    /// Check if transitive
    pub fn is_transitive(&self) -> bool {
        self.characteristics
            .contains(&PropertyCharacteristic::Transitive)
    }

    /// Check if symmetric
    pub fn is_symmetric(&self) -> bool {
        self.characteristics
            .contains(&PropertyCharacteristic::Symmetric)
    }
}

// ============================================================================
// INDIVIDUALS
// ============================================================================

/// An individual (instance)
#[derive(Debug, Clone)]
pub struct Individual {
    /// Individual ID
    pub id: IndividualId,
    /// IRI/name
    pub iri: String,
    /// Label
    pub label: Option<String>,
    /// Asserted types (classes)
    pub types: Vec<ClassId>,
    /// Object property values
    pub object_properties: BTreeMap<PropertyId, Vec<IndividualId>>,
    /// Data property values
    pub data_properties: BTreeMap<PropertyId, Vec<DataValue>>,
    /// Same as (owl:sameAs)
    pub same_as: Vec<IndividualId>,
    /// Different from (owl:differentFrom)
    pub different_from: Vec<IndividualId>,
}

impl Individual {
    /// Create a new individual
    pub fn new(id: IndividualId, iri: String) -> Self {
        Self {
            id,
            iri,
            label: None,
            types: Vec::new(),
            object_properties: BTreeMap::new(),
            data_properties: BTreeMap::new(),
            same_as: Vec::new(),
            different_from: Vec::new(),
        }
    }

    /// Add a type
    pub fn add_type(&mut self, class: ClassId) {
        if !self.types.contains(&class) {
            self.types.push(class);
        }
    }

    /// Add an object property value
    pub fn add_object_property(&mut self, property: PropertyId, value: IndividualId) {
        self.object_properties
            .entry(property)
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Add a data property value
    pub fn add_data_property(&mut self, property: PropertyId, value: DataValue) {
        self.data_properties
            .entry(property)
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Get object property values
    pub fn get_object_property(&self, property: PropertyId) -> &[IndividualId] {
        self.object_properties
            .get(&property)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get data property values
    pub fn get_data_property(&self, property: PropertyId) -> &[DataValue] {
        self.data_properties
            .get(&property)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

// ============================================================================
// ONTOLOGY
// ============================================================================

/// The main ontology structure
pub struct Ontology {
    /// Ontology IRI
    pub iri: String,
    /// Version IRI
    pub version: Option<String>,
    /// Classes
    classes: BTreeMap<ClassId, OntologyClass>,
    /// Properties
    properties: BTreeMap<PropertyId, OntologyProperty>,
    /// Individuals
    individuals: BTreeMap<IndividualId, Individual>,
    /// Class name to ID mapping
    class_names: BTreeMap<String, ClassId>,
    /// Property name to ID mapping
    property_names: BTreeMap<String, PropertyId>,
    /// Individual name to ID mapping
    individual_names: BTreeMap<String, IndividualId>,
    /// Next class ID
    next_class_id: u32,
    /// Next property ID
    next_property_id: u32,
    /// Next individual ID
    next_individual_id: u32,
    /// Computed class hierarchy cache
    class_hierarchy: BTreeMap<ClassId, ClassHierarchyInfo>,
    /// Is hierarchy computed?
    hierarchy_valid: bool,
}

/// Cached class hierarchy information
#[derive(Debug, Clone, Default)]
struct ClassHierarchyInfo {
    /// All superclasses (transitive closure)
    all_superclasses: BTreeSet<ClassId>,
    /// All subclasses (transitive closure)
    all_subclasses: BTreeSet<ClassId>,
    /// Direct instances
    direct_instances: Vec<IndividualId>,
    /// All instances (including subclass instances)
    all_instances: Vec<IndividualId>,
}

impl Ontology {
    /// Create a new ontology
    pub fn new(iri: String) -> Self {
        let mut ontology = Self {
            iri,
            version: None,
            classes: BTreeMap::new(),
            properties: BTreeMap::new(),
            individuals: BTreeMap::new(),
            class_names: BTreeMap::new(),
            property_names: BTreeMap::new(),
            individual_names: BTreeMap::new(),
            next_class_id: 2, // 0 = Thing, 1 = Nothing
            next_property_id: 0,
            next_individual_id: 0,
            class_hierarchy: BTreeMap::new(),
            hierarchy_valid: false,
        };

        // Add built-in classes
        let thing = OntologyClass::new(ClassId::THING, String::from("owl:Thing"));
        let nothing = OntologyClass::new(ClassId::NOTHING, String::from("owl:Nothing"));

        ontology.classes.insert(ClassId::THING, thing);
        ontology.classes.insert(ClassId::NOTHING, nothing);
        ontology
            .class_names
            .insert(String::from("owl:Thing"), ClassId::THING);
        ontology
            .class_names
            .insert(String::from("owl:Nothing"), ClassId::NOTHING);

        ontology
    }

    // ========================================================================
    // CLASS OPERATIONS
    // ========================================================================

    /// Add a new class
    pub fn add_class(&mut self, iri: String) -> ClassId {
        if let Some(&id) = self.class_names.get(&iri) {
            return id;
        }

        let id = ClassId(self.next_class_id);
        self.next_class_id += 1;

        let mut class = OntologyClass::new(id, iri.clone());
        class.add_superclass(ClassId::THING); // All classes are subclasses of Thing

        self.class_names.insert(iri, id);
        self.classes.insert(id, class);
        self.hierarchy_valid = false;

        id
    }

    /// Add a subclass relationship
    pub fn add_subclass(&mut self, subclass: ClassId, superclass: ClassId) {
        if let Some(class) = self.classes.get_mut(&subclass) {
            class.add_superclass(superclass);
            self.hierarchy_valid = false;
        }
    }

    /// Add disjoint classes
    pub fn add_disjoint(&mut self, class1: ClassId, class2: ClassId) {
        if let Some(c) = self.classes.get_mut(&class1) {
            c.add_disjoint(class2);
        }
        if let Some(c) = self.classes.get_mut(&class2) {
            c.add_disjoint(class1);
        }
    }

    /// Get a class by ID
    pub fn get_class(&self, id: ClassId) -> Option<&OntologyClass> {
        self.classes.get(&id)
    }

    /// Get a class by name
    pub fn get_class_by_name(&self, name: &str) -> Option<&OntologyClass> {
        self.class_names
            .get(name)
            .and_then(|id| self.classes.get(id))
    }

    /// Get all direct superclasses
    pub fn get_superclasses(&self, class: ClassId) -> Vec<ClassId> {
        self.classes
            .get(&class)
            .map(|c| c.superclasses.clone())
            .unwrap_or_default()
    }

    /// Get all superclasses (transitive)
    pub fn get_all_superclasses(&mut self, class: ClassId) -> BTreeSet<ClassId> {
        self.ensure_hierarchy();
        self.class_hierarchy
            .get(&class)
            .map(|h| h.all_superclasses.clone())
            .unwrap_or_default()
    }

    /// Get all subclasses (transitive)
    pub fn get_all_subclasses(&mut self, class: ClassId) -> BTreeSet<ClassId> {
        self.ensure_hierarchy();
        self.class_hierarchy
            .get(&class)
            .map(|h| h.all_subclasses.clone())
            .unwrap_or_default()
    }

    // ========================================================================
    // PROPERTY OPERATIONS
    // ========================================================================

    /// Add an object property
    pub fn add_object_property(&mut self, iri: String) -> PropertyId {
        if let Some(&id) = self.property_names.get(&iri) {
            return id;
        }

        let id = PropertyId(self.next_property_id);
        self.next_property_id += 1;

        let property = OntologyProperty::new_object(id, iri.clone());
        self.property_names.insert(iri, id);
        self.properties.insert(id, property);

        id
    }

    /// Add a data property
    pub fn add_data_property(&mut self, iri: String, datatype: DataType) -> PropertyId {
        if let Some(&id) = self.property_names.get(&iri) {
            return id;
        }

        let id = PropertyId(self.next_property_id);
        self.next_property_id += 1;

        let property = OntologyProperty::new_data(id, iri.clone(), datatype);
        self.property_names.insert(iri, id);
        self.properties.insert(id, property);

        id
    }

    /// Set property domain
    pub fn set_property_domain(&mut self, property: PropertyId, domain: ClassId) {
        if let Some(p) = self.properties.get_mut(&property) {
            if !p.domain.contains(&domain) {
                p.domain.push(domain);
            }
        }
    }

    /// Set property range (for object properties)
    pub fn set_property_range(&mut self, property: PropertyId, range: ClassId) {
        if let Some(p) = self.properties.get_mut(&property) {
            if !p.range_classes.contains(&range) {
                p.range_classes.push(range);
            }
        }
    }

    /// Set inverse property
    pub fn set_inverse_property(&mut self, property: PropertyId, inverse: PropertyId) {
        if let Some(p) = self.properties.get_mut(&property) {
            p.inverse_of = Some(inverse);
        }
        if let Some(p) = self.properties.get_mut(&inverse) {
            p.inverse_of = Some(property);
        }
    }

    /// Make property transitive
    pub fn make_transitive(&mut self, property: PropertyId) {
        if let Some(p) = self.properties.get_mut(&property) {
            p.add_characteristic(PropertyCharacteristic::Transitive);
        }
    }

    /// Make property symmetric
    pub fn make_symmetric(&mut self, property: PropertyId) {
        if let Some(p) = self.properties.get_mut(&property) {
            p.add_characteristic(PropertyCharacteristic::Symmetric);
        }
    }

    /// Make property functional
    pub fn make_functional(&mut self, property: PropertyId) {
        if let Some(p) = self.properties.get_mut(&property) {
            p.add_characteristic(PropertyCharacteristic::Functional);
        }
    }

    /// Get a property by ID
    pub fn get_property(&self, id: PropertyId) -> Option<&OntologyProperty> {
        self.properties.get(&id)
    }

    // ========================================================================
    // INDIVIDUAL OPERATIONS
    // ========================================================================

    /// Add an individual
    pub fn add_individual(&mut self, iri: String) -> IndividualId {
        if let Some(&id) = self.individual_names.get(&iri) {
            return id;
        }

        let id = IndividualId(self.next_individual_id);
        self.next_individual_id += 1;

        let individual = Individual::new(id, iri.clone());
        self.individual_names.insert(iri, id);
        self.individuals.insert(id, individual);
        self.hierarchy_valid = false;

        id
    }

    /// Assert that an individual is of a type
    pub fn add_type_assertion(&mut self, individual: IndividualId, class: ClassId) {
        if let Some(ind) = self.individuals.get_mut(&individual) {
            ind.add_type(class);
            self.hierarchy_valid = false;
        }
    }

    /// Add an object property assertion
    pub fn add_object_property_assertion(
        &mut self,
        subject: IndividualId,
        property: PropertyId,
        object: IndividualId,
    ) {
        if let Some(ind) = self.individuals.get_mut(&subject) {
            ind.add_object_property(property, object);
        }

        // Handle symmetric properties
        if let Some(prop) = self.properties.get(&property) {
            if prop.is_symmetric() {
                if let Some(obj) = self.individuals.get_mut(&object) {
                    obj.add_object_property(property, subject);
                }
            }
        }
    }

    /// Add a data property assertion
    pub fn add_data_property_assertion(
        &mut self,
        subject: IndividualId,
        property: PropertyId,
        value: DataValue,
    ) {
        if let Some(ind) = self.individuals.get_mut(&subject) {
            ind.add_data_property(property, value);
        }
    }

    /// Get an individual by ID
    pub fn get_individual(&self, id: IndividualId) -> Option<&Individual> {
        self.individuals.get(&id)
    }

    /// Get all individuals of a class (including subclass instances)
    pub fn get_instances(&mut self, class: ClassId) -> Vec<IndividualId> {
        self.ensure_hierarchy();
        self.class_hierarchy
            .get(&class)
            .map(|h| h.all_instances.clone())
            .unwrap_or_default()
    }

    // ========================================================================
    // REASONING
    // ========================================================================

    /// Ensure hierarchy is computed
    fn ensure_hierarchy(&mut self) {
        if self.hierarchy_valid {
            return;
        }

        self.compute_hierarchy();
        self.hierarchy_valid = true;
    }

    /// Compute class hierarchy
    fn compute_hierarchy(&mut self) {
        // Clear existing
        self.class_hierarchy.clear();

        // Initialize all classes
        for &id in self.classes.keys() {
            self.class_hierarchy
                .insert(id, ClassHierarchyInfo::default());
        }

        // Compute transitive superclasses for each class
        for &id in self.classes.keys() {
            let supers = self.compute_all_superclasses(id);
            if let Some(info) = self.class_hierarchy.get_mut(&id) {
                info.all_superclasses = supers;
            }
        }

        // Compute subclasses (inverse of superclasses)
        let class_ids: Vec<ClassId> = self.classes.keys().copied().collect();
        for &id in &class_ids {
            let supers = self
                .class_hierarchy
                .get(&id)
                .map(|h| h.all_superclasses.clone())
                .unwrap_or_default();

            for sup in supers {
                if let Some(info) = self.class_hierarchy.get_mut(&sup) {
                    info.all_subclasses.insert(id);
                }
            }
        }

        // Compute instances
        for (&ind_id, individual) in &self.individuals {
            for &class in &individual.types {
                // Direct instance
                if let Some(info) = self.class_hierarchy.get_mut(&class) {
                    info.direct_instances.push(ind_id);
                    info.all_instances.push(ind_id);
                }

                // Also instance of all superclasses
                let supers = self
                    .class_hierarchy
                    .get(&class)
                    .map(|h| h.all_superclasses.clone())
                    .unwrap_or_default();

                for sup in supers {
                    if let Some(info) = self.class_hierarchy.get_mut(&sup) {
                        if !info.all_instances.contains(&ind_id) {
                            info.all_instances.push(ind_id);
                        }
                    }
                }
            }
        }
    }

    /// Compute all superclasses (transitive) for a class
    fn compute_all_superclasses(&self, class: ClassId) -> BTreeSet<ClassId> {
        let mut result = BTreeSet::new();
        let mut queue = vec![class];
        let mut depth = 0;

        while let Some(current) = queue.pop() {
            depth += 1;
            if depth > MAX_INHERITANCE_DEPTH {
                break;
            }

            if let Some(cls) = self.classes.get(&current) {
                for &sup in &cls.superclasses {
                    if !result.contains(&sup) {
                        result.insert(sup);
                        queue.push(sup);
                    }
                }
            }
        }

        result
    }

    /// Check if class1 is subclass of class2
    pub fn is_subclass_of(&mut self, class1: ClassId, class2: ClassId) -> bool {
        if class1 == class2 {
            return true;
        }
        if class2 == ClassId::THING {
            return true;
        }
        if class1 == ClassId::NOTHING {
            return true;
        }

        self.ensure_hierarchy();
        self.class_hierarchy
            .get(&class1)
            .map(|h| h.all_superclasses.contains(&class2))
            .unwrap_or(false)
    }

    /// Check if an individual is an instance of a class
    pub fn is_instance_of(&mut self, individual: IndividualId, class: ClassId) -> bool {
        // Direct type check
        if let Some(ind) = self.individuals.get(&individual) {
            if ind.types.contains(&class) {
                return true;
            }

            // Check superclasses
            for &type_class in &ind.types {
                if self.is_subclass_of(type_class, class) {
                    return true;
                }
            }
        }

        false
    }

    /// Classify an individual (find all types)
    pub fn classify(&mut self, individual: IndividualId) -> Vec<ClassId> {
        let mut types = Vec::new();

        if let Some(ind) = self.individuals.get(&individual) {
            // Add asserted types
            for &t in &ind.types {
                types.push(t);
            }

            // Add inferred superclass types
            let asserted = types.clone();
            for class in asserted {
                let supers = self.compute_all_superclasses(class);
                for sup in supers {
                    if !types.contains(&sup) {
                        types.push(sup);
                    }
                }
            }
        }

        types
    }

    /// Check consistency (basic check)
    pub fn is_consistent(&mut self) -> bool {
        self.ensure_hierarchy();

        // Check that no individual is instance of disjoint classes
        for individual in self.individuals.values() {
            let types = &individual.types;

            for i in 0..types.len() {
                for j in (i + 1)..types.len() {
                    if let Some(class) = self.classes.get(&types[i]) {
                        if class.disjoint_with.contains(&types[j]) {
                            return false;
                        }
                    }
                }
            }
        }

        // Check Nothing has no instances
        if let Some(info) = self.class_hierarchy.get(&ClassId::NOTHING) {
            if !info.all_instances.is_empty() {
                return false;
            }
        }

        true
    }

    /// Find most specific classes for an individual
    pub fn most_specific_types(&mut self, individual: IndividualId) -> Vec<ClassId> {
        let all_types = self.classify(individual);
        let mut most_specific = Vec::new();

        for &t in &all_types {
            let mut is_most_specific = true;

            for &other in &all_types {
                if other != t && self.is_subclass_of(other, t) {
                    // other is more specific than t
                    is_most_specific = false;
                    break;
                }
            }

            if is_most_specific {
                most_specific.push(t);
            }
        }

        most_specific
    }
}

// ============================================================================
// KERNEL ONTOLOGY
// ============================================================================

/// Pre-built kernel domain ontology
pub struct KernelOntology {
    /// The ontology
    pub ontology: Ontology,
    /// Pre-defined class IDs
    pub classes: KernelClasses,
    /// Pre-defined property IDs
    pub properties: KernelProperties,
}

/// Kernel domain classes
pub struct KernelClasses {
    /// KernelEntity - top level
    pub kernel_entity: ClassId,
    /// Process
    pub process: ClassId,
    /// Thread
    pub thread: ClassId,
    /// Task
    pub task: ClassId,
    /// MemoryRegion
    pub memory_region: ClassId,
    /// FileDescriptor
    pub file_descriptor: ClassId,
    /// Device
    pub device: ClassId,
    /// Driver
    pub driver: ClassId,
    /// Module
    pub module: ClassId,
    /// Lock
    pub lock: ClassId,
    /// Mutex
    pub mutex: ClassId,
    /// Semaphore
    pub semaphore: ClassId,
    /// Socket
    pub socket: ClassId,
    /// Pipe
    pub pipe: ClassId,
    /// Interrupt
    pub interrupt: ClassId,
    /// CpuCore
    pub cpu_core: ClassId,
    /// NumaNode
    pub numa_node: ClassId,
    /// User
    pub user: ClassId,
    /// Group
    pub group: ClassId,
    /// Namespace
    pub namespace: ClassId,
    /// Cgroup
    pub cgroup: ClassId,
}

/// Kernel domain properties
pub struct KernelProperties {
    /// parentProcess
    pub parent_process: PropertyId,
    /// childProcess
    pub child_process: PropertyId,
    /// ownsResource
    pub owns_resource: PropertyId,
    /// ownedBy
    pub owned_by: PropertyId,
    /// usesResource
    pub uses_resource: PropertyId,
    /// dependsOn
    pub depends_on: PropertyId,
    /// holdsLock
    pub holds_lock: PropertyId,
    /// waitsForLock
    pub waits_for_lock: PropertyId,
    /// runsOnCpu
    pub runs_on_cpu: PropertyId,
    /// belongsToNamespace
    pub belongs_to_namespace: PropertyId,
    /// belongsToCgroup
    pub belongs_to_cgroup: PropertyId,
    /// hasPid (data property)
    pub has_pid: PropertyId,
    /// hasPriority (data property)
    pub has_priority: PropertyId,
    /// hasMemoryUsage (data property)
    pub has_memory_usage: PropertyId,
}

impl KernelOntology {
    /// Create the kernel ontology
    pub fn new() -> Self {
        let mut ontology = Ontology::new(String::from("http://helix.os/kernel/ontology"));

        // Create classes
        let kernel_entity = ontology.add_class(String::from("KernelEntity"));

        // Process hierarchy
        let process = ontology.add_class(String::from("Process"));
        ontology.add_subclass(process, kernel_entity);

        let thread = ontology.add_class(String::from("Thread"));
        ontology.add_subclass(thread, kernel_entity);

        let task = ontology.add_class(String::from("Task"));
        ontology.add_subclass(task, kernel_entity);
        ontology.add_subclass(process, task);
        ontology.add_subclass(thread, task);

        // Memory
        let memory_region = ontology.add_class(String::from("MemoryRegion"));
        ontology.add_subclass(memory_region, kernel_entity);

        // File system
        let file_descriptor = ontology.add_class(String::from("FileDescriptor"));
        ontology.add_subclass(file_descriptor, kernel_entity);

        // Devices
        let device = ontology.add_class(String::from("Device"));
        ontology.add_subclass(device, kernel_entity);

        let driver = ontology.add_class(String::from("Driver"));
        ontology.add_subclass(driver, kernel_entity);

        let module = ontology.add_class(String::from("Module"));
        ontology.add_subclass(module, kernel_entity);

        // Synchronization
        let lock = ontology.add_class(String::from("Lock"));
        ontology.add_subclass(lock, kernel_entity);

        let mutex = ontology.add_class(String::from("Mutex"));
        ontology.add_subclass(mutex, lock);

        let semaphore = ontology.add_class(String::from("Semaphore"));
        ontology.add_subclass(semaphore, lock);

        // IPC
        let socket = ontology.add_class(String::from("Socket"));
        ontology.add_subclass(socket, kernel_entity);

        let pipe = ontology.add_class(String::from("Pipe"));
        ontology.add_subclass(pipe, kernel_entity);

        // Hardware
        let interrupt = ontology.add_class(String::from("Interrupt"));
        ontology.add_subclass(interrupt, kernel_entity);

        let cpu_core = ontology.add_class(String::from("CpuCore"));
        ontology.add_subclass(cpu_core, kernel_entity);

        let numa_node = ontology.add_class(String::from("NumaNode"));
        ontology.add_subclass(numa_node, kernel_entity);

        // Users/Groups
        let user = ontology.add_class(String::from("User"));
        ontology.add_subclass(user, kernel_entity);

        let group = ontology.add_class(String::from("Group"));
        ontology.add_subclass(group, kernel_entity);

        // Namespaces/Cgroups
        let namespace = ontology.add_class(String::from("Namespace"));
        ontology.add_subclass(namespace, kernel_entity);

        let cgroup = ontology.add_class(String::from("Cgroup"));
        ontology.add_subclass(cgroup, kernel_entity);

        // Disjoint classes
        ontology.add_disjoint(process, memory_region);
        ontology.add_disjoint(process, device);
        ontology.add_disjoint(thread, device);
        ontology.add_disjoint(user, process);

        // Create properties
        let parent_process = ontology.add_object_property(String::from("parentProcess"));
        ontology.set_property_domain(parent_process, process);
        ontology.set_property_range(parent_process, process);

        let child_process = ontology.add_object_property(String::from("childProcess"));
        ontology.set_property_domain(child_process, process);
        ontology.set_property_range(child_process, process);
        ontology.set_inverse_property(parent_process, child_process);

        let owns_resource = ontology.add_object_property(String::from("ownsResource"));
        ontology.set_property_domain(owns_resource, task);

        let owned_by = ontology.add_object_property(String::from("ownedBy"));
        ontology.set_property_range(owned_by, task);
        ontology.set_inverse_property(owns_resource, owned_by);

        let uses_resource = ontology.add_object_property(String::from("usesResource"));
        ontology.set_property_domain(uses_resource, task);

        let depends_on = ontology.add_object_property(String::from("dependsOn"));
        ontology.make_transitive(depends_on);

        let holds_lock = ontology.add_object_property(String::from("holdsLock"));
        ontology.set_property_domain(holds_lock, task);
        ontology.set_property_range(holds_lock, lock);

        let waits_for_lock = ontology.add_object_property(String::from("waitsForLock"));
        ontology.set_property_domain(waits_for_lock, task);
        ontology.set_property_range(waits_for_lock, lock);

        let runs_on_cpu = ontology.add_object_property(String::from("runsOnCpu"));
        ontology.set_property_domain(runs_on_cpu, task);
        ontology.set_property_range(runs_on_cpu, cpu_core);
        ontology.make_functional(runs_on_cpu);

        let belongs_to_namespace = ontology.add_object_property(String::from("belongsToNamespace"));
        ontology.set_property_domain(belongs_to_namespace, task);
        ontology.set_property_range(belongs_to_namespace, namespace);

        let belongs_to_cgroup = ontology.add_object_property(String::from("belongsToCgroup"));
        ontology.set_property_domain(belongs_to_cgroup, task);
        ontology.set_property_range(belongs_to_cgroup, cgroup);

        // Data properties
        let has_pid = ontology.add_data_property(String::from("hasPid"), DataType::Integer);
        ontology.set_property_domain(has_pid, process);
        ontology.make_functional(has_pid);

        let has_priority =
            ontology.add_data_property(String::from("hasPriority"), DataType::Integer);
        ontology.set_property_domain(has_priority, task);

        let has_memory_usage =
            ontology.add_data_property(String::from("hasMemoryUsage"), DataType::Integer);
        ontology.set_property_domain(has_memory_usage, task);

        let classes = KernelClasses {
            kernel_entity,
            process,
            thread,
            task,
            memory_region,
            file_descriptor,
            device,
            driver,
            module,
            lock,
            mutex,
            semaphore,
            socket,
            pipe,
            interrupt,
            cpu_core,
            numa_node,
            user,
            group,
            namespace,
            cgroup,
        };

        let properties = KernelProperties {
            parent_process,
            child_process,
            owns_resource,
            owned_by,
            uses_resource,
            depends_on,
            holds_lock,
            waits_for_lock,
            runs_on_cpu,
            belongs_to_namespace,
            belongs_to_cgroup,
            has_pid,
            has_priority,
            has_memory_usage,
        };

        Self {
            ontology,
            classes,
            properties,
        }
    }

    /// Add a process to the ontology
    pub fn add_process(&mut self, name: &str, pid: i64) -> IndividualId {
        let id = self.ontology.add_individual(String::from(name));
        self.ontology.add_type_assertion(id, self.classes.process);
        self.ontology.add_data_property_assertion(
            id,
            self.properties.has_pid,
            DataValue::Integer(pid),
        );
        id
    }

    /// Add a thread
    pub fn add_thread(&mut self, name: &str, parent: IndividualId) -> IndividualId {
        let id = self.ontology.add_individual(String::from(name));
        self.ontology.add_type_assertion(id, self.classes.thread);
        self.ontology
            .add_object_property_assertion(parent, self.properties.child_process, id);
        id
    }

    /// Add a lock
    pub fn add_lock(&mut self, name: &str) -> IndividualId {
        let id = self.ontology.add_individual(String::from(name));
        self.ontology.add_type_assertion(id, self.classes.lock);
        id
    }

    /// Record lock hold
    pub fn record_lock_held(&mut self, holder: IndividualId, lock: IndividualId) {
        self.ontology
            .add_object_property_assertion(holder, self.properties.holds_lock, lock);
    }

    /// Record lock wait
    pub fn record_lock_wait(&mut self, waiter: IndividualId, lock: IndividualId) {
        self.ontology
            .add_object_property_assertion(waiter, self.properties.waits_for_lock, lock);
    }

    /// Query all processes
    pub fn get_all_processes(&mut self) -> Vec<IndividualId> {
        self.ontology.get_instances(self.classes.process)
    }

    /// Query all tasks (processes and threads)
    pub fn get_all_tasks(&mut self) -> Vec<IndividualId> {
        self.ontology.get_instances(self.classes.task)
    }

    /// Check if entity is a process
    pub fn is_process(&mut self, entity: IndividualId) -> bool {
        self.ontology.is_instance_of(entity, self.classes.process)
    }

    /// Check if entity is a task
    pub fn is_task(&mut self, entity: IndividualId) -> bool {
        self.ontology.is_instance_of(entity, self.classes.task)
    }
}

impl Default for KernelOntology {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_hierarchy() {
        let mut ontology = Ontology::new(String::from("test"));

        let animal = ontology.add_class(String::from("Animal"));
        let mammal = ontology.add_class(String::from("Mammal"));
        let dog = ontology.add_class(String::from("Dog"));

        ontology.add_subclass(mammal, animal);
        ontology.add_subclass(dog, mammal);

        // Dog should be subclass of Mammal and Animal
        assert!(ontology.is_subclass_of(dog, mammal));
        assert!(ontology.is_subclass_of(dog, animal));
        assert!(ontology.is_subclass_of(mammal, animal));

        // But not the other way
        assert!(!ontology.is_subclass_of(animal, dog));
    }

    #[test]
    fn test_individuals() {
        let mut ontology = Ontology::new(String::from("test"));

        let animal = ontology.add_class(String::from("Animal"));
        let dog = ontology.add_class(String::from("Dog"));
        ontology.add_subclass(dog, animal);

        let rex = ontology.add_individual(String::from("Rex"));
        ontology.add_type_assertion(rex, dog);

        // Rex is a Dog
        assert!(ontology.is_instance_of(rex, dog));

        // Rex is also an Animal (by inheritance)
        assert!(ontology.is_instance_of(rex, animal));

        // Instances
        let dogs = ontology.get_instances(dog);
        assert!(dogs.contains(&rex));

        let animals = ontology.get_instances(animal);
        assert!(animals.contains(&rex));
    }

    #[test]
    fn test_disjoint_classes() {
        let mut ontology = Ontology::new(String::from("test"));

        let cat = ontology.add_class(String::from("Cat"));
        let dog = ontology.add_class(String::from("Dog"));

        ontology.add_disjoint(cat, dog);

        // Create individual that is both
        let thing = ontology.add_individual(String::from("Thing"));
        ontology.add_type_assertion(thing, cat);
        ontology.add_type_assertion(thing, dog);

        // Should be inconsistent
        assert!(!ontology.is_consistent());
    }

    #[test]
    fn test_kernel_ontology() {
        let mut kernel = KernelOntology::new();

        // Add some entities
        let init = kernel.add_process("init", 1);
        let bash = kernel.add_process("bash", 100);
        let thread1 = kernel.add_thread("bash_thread1", bash);

        // Check types
        assert!(kernel.is_process(init));
        assert!(kernel.is_process(bash));
        assert!(!kernel.is_process(thread1));

        // All are tasks
        assert!(kernel.is_task(init));
        assert!(kernel.is_task(bash));
        assert!(kernel.is_task(thread1));

        // Get all processes
        let processes = kernel.get_all_processes();
        assert!(processes.contains(&init));
        assert!(processes.contains(&bash));

        // Get all tasks
        let tasks = kernel.get_all_tasks();
        assert!(tasks.contains(&init));
        assert!(tasks.contains(&bash));
        assert!(tasks.contains(&thread1));
    }

    #[test]
    fn test_properties() {
        let mut ontology = Ontology::new(String::from("test"));

        let person = ontology.add_class(String::from("Person"));
        let knows = ontology.add_object_property(String::from("knows"));
        ontology.make_symmetric(knows);

        let alice = ontology.add_individual(String::from("Alice"));
        let bob = ontology.add_individual(String::from("Bob"));

        ontology.add_type_assertion(alice, person);
        ontology.add_type_assertion(bob, person);

        // Alice knows Bob
        ontology.add_object_property_assertion(alice, knows, bob);

        // Due to symmetry, Bob should know Alice too
        if let Some(bob_ind) = ontology.get_individual(bob) {
            let knows_list = bob_ind.get_object_property(knows);
            assert!(knows_list.contains(&alice));
        }
    }

    #[test]
    fn test_classification() {
        let mut ontology = Ontology::new(String::from("test"));

        let entity = ontology.add_class(String::from("Entity"));
        let resource = ontology.add_class(String::from("Resource"));
        let memory = ontology.add_class(String::from("Memory"));

        ontology.add_subclass(resource, entity);
        ontology.add_subclass(memory, resource);

        let heap = ontology.add_individual(String::from("Heap"));
        ontology.add_type_assertion(heap, memory);

        let types = ontology.classify(heap);

        // Should include Memory, Resource, Entity, Thing
        assert!(types.contains(&memory));
        assert!(types.contains(&resource));
        assert!(types.contains(&entity));
        assert!(types.contains(&ClassId::THING));

        // Most specific should be Memory
        let most_specific = ontology.most_specific_types(heap);
        assert!(most_specific.contains(&memory));
        assert!(!most_specific.contains(&resource));
    }
}
