//! NEXUS Year 2: Semantic Understanding Module
//!
//! Provides semantic embeddings, similarity metrics, concept spaces,
//! and knowledge representation for deep understanding.
//!
//! # Submodules
//!
//! - `embeddings`: Vector embeddings for concepts and entities
//! - `similarity`: Similarity metrics and comparisons
//! - `concepts`: Concept spaces and hierarchies
//! - `knowledge`: Knowledge representation and linking

extern crate alloc;

pub mod concepts;
pub mod embeddings;
pub mod knowledge;
pub mod similarity;

// Re-export key types
pub use concepts::{Concept, ConceptHierarchy, ConceptId, ConceptRelation, ConceptSpace};
pub use embeddings::{Embedding, EmbeddingDecoder, EmbeddingEncoder, EmbeddingId, EmbeddingSpace};
pub use knowledge::{Entity, EntityId, KnowledgeBase, KnowledgeQuery, Relation, RelationId};
pub use similarity::{
    CosineSimilarity, EuclideanDistance, SimilarityMatrix, SimilarityMetric, SimilaritySearch,
};
