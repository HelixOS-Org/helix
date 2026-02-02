//! # Sense Preprocessing
//!
//! Preprocesses sensory input for downstream processing.
//! Implements normalization, tokenization, and feature extraction.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PREPROCESSING TYPES
// ============================================================================

/// Raw input
#[derive(Debug, Clone)]
pub struct RawInput {
    /// Input ID
    pub id: u64,
    /// Source
    pub source: String,
    /// Content type
    pub content_type: ContentType,
    /// Data
    pub data: InputData,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Content type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Text,
    Code,
    Numeric,
    Binary,
    Structured,
}

/// Input data
#[derive(Debug, Clone)]
pub enum InputData {
    Text(String),
    Bytes(Vec<u8>),
    Numbers(Vec<f64>),
    Tokens(Vec<Token>),
    Structured(BTreeMap<String, InputData>),
}

/// Token
#[derive(Debug, Clone)]
pub struct Token {
    /// Token ID
    pub id: u64,
    /// Type
    pub token_type: TokenType,
    /// Text
    pub text: String,
    /// Position
    pub position: Position,
    /// Features
    pub features: BTreeMap<String, f64>,
}

/// Token type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Keyword,
    Identifier,
    Operator,
    Literal,
    Punctuation,
    Whitespace,
    Comment,
    Unknown,
}

/// Position
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
    pub length: usize,
}

/// Preprocessed output
#[derive(Debug, Clone)]
pub struct PreprocessedOutput {
    /// Output ID
    pub id: u64,
    /// Source input ID
    pub source_id: u64,
    /// Tokens
    pub tokens: Vec<Token>,
    /// Features
    pub features: FeatureSet,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Feature set
#[derive(Debug, Clone)]
pub struct FeatureSet {
    /// Numeric features
    pub numeric: BTreeMap<String, f64>,
    /// Categorical features
    pub categorical: BTreeMap<String, String>,
    /// Vector features
    pub vectors: BTreeMap<String, Vec<f64>>,
}

impl Default for FeatureSet {
    fn default() -> Self {
        Self {
            numeric: BTreeMap::new(),
            categorical: BTreeMap::new(),
            vectors: BTreeMap::new(),
        }
    }
}

/// Preprocessing pipeline
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Pipeline ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Steps
    pub steps: Vec<PreprocessStep>,
}

/// Preprocessing step
#[derive(Debug, Clone)]
pub enum PreprocessStep {
    Normalize(NormalizeConfig),
    Tokenize(TokenizeConfig),
    Filter(FilterConfig),
    Extract(ExtractConfig),
    Transform(TransformConfig),
}

/// Normalize configuration
#[derive(Debug, Clone)]
pub struct NormalizeConfig {
    pub lowercase: bool,
    pub trim: bool,
    pub collapse_whitespace: bool,
    pub remove_accents: bool,
}

/// Tokenize configuration
#[derive(Debug, Clone)]
pub struct TokenizeConfig {
    pub mode: TokenizeMode,
    pub preserve_case: bool,
    pub include_positions: bool,
}

/// Tokenize mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizeMode {
    Whitespace,
    Word,
    Character,
    CodeAware,
}

/// Filter configuration
#[derive(Debug, Clone)]
pub struct FilterConfig {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub exclude_types: Vec<TokenType>,
    pub exclude_patterns: Vec<String>,
}

/// Extract configuration
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    pub features: Vec<String>,
    pub n_grams: Option<usize>,
    pub tf_idf: bool,
}

/// Transform configuration
#[derive(Debug, Clone)]
pub struct TransformConfig {
    pub operation: TransformOp,
    pub target: String,
}

/// Transform operation
#[derive(Debug, Clone)]
pub enum TransformOp {
    Scale { min: f64, max: f64 },
    ZScore,
    Log,
    Embed { dimensions: usize },
}

// ============================================================================
// PREPROCESSOR
// ============================================================================

/// Preprocessor
pub struct Preprocessor {
    /// Pipelines
    pipelines: BTreeMap<u64, Pipeline>,
    /// Default pipeline
    default_pipeline: Option<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: PreprocessorConfig,
    /// Statistics
    stats: PreprocessorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct PreprocessorConfig {
    /// Max tokens
    pub max_tokens: usize,
    /// Default tokenize mode
    pub default_tokenize: TokenizeMode,
}

impl Default for PreprocessorConfig {
    fn default() -> Self {
        Self {
            max_tokens: 10000,
            default_tokenize: TokenizeMode::Word,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct PreprocessorStats {
    /// Inputs processed
    pub inputs_processed: u64,
    /// Tokens generated
    pub tokens_generated: u64,
    /// Features extracted
    pub features_extracted: u64,
}

impl Preprocessor {
    /// Create new preprocessor
    pub fn new(config: PreprocessorConfig) -> Self {
        Self {
            pipelines: BTreeMap::new(),
            default_pipeline: None,
            next_id: AtomicU64::new(1),
            config,
            stats: PreprocessorStats::default(),
        }
    }

    /// Create pipeline
    pub fn create_pipeline(&mut self, name: &str, steps: Vec<PreprocessStep>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let pipeline = Pipeline {
            id,
            name: name.into(),
            steps,
        };

        self.pipelines.insert(id, pipeline);

        if self.default_pipeline.is_none() {
            self.default_pipeline = Some(id);
        }

        id
    }

    /// Set default pipeline
    pub fn set_default(&mut self, pipeline_id: u64) {
        if self.pipelines.contains_key(&pipeline_id) {
            self.default_pipeline = Some(pipeline_id);
        }
    }

    /// Process input
    pub fn process(&mut self, input: RawInput, pipeline_id: Option<u64>) -> Option<PreprocessedOutput> {
        let pid = pipeline_id.or(self.default_pipeline)?;
        let pipeline = self.pipelines.get(&pid)?.clone();

        self.stats.inputs_processed += 1;

        let mut tokens = Vec::new();
        let mut features = FeatureSet::default();
        let mut text = match &input.data {
            InputData::Text(t) => t.clone(),
            InputData::Bytes(b) => String::from_utf8_lossy(b).into(),
            _ => String::new(),
        };

        for step in &pipeline.steps {
            match step {
                PreprocessStep::Normalize(config) => {
                    text = self.normalize(&text, config);
                }
                PreprocessStep::Tokenize(config) => {
                    tokens = self.tokenize(&text, config);
                }
                PreprocessStep::Filter(config) => {
                    tokens = self.filter(tokens, config);
                }
                PreprocessStep::Extract(config) => {
                    features = self.extract(&tokens, config);
                }
                PreprocessStep::Transform(config) => {
                    self.transform(&mut features, config);
                }
            }
        }

        self.stats.tokens_generated += tokens.len() as u64;
        self.stats.features_extracted += features.numeric.len() as u64;

        let output_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        Some(PreprocessedOutput {
            id: output_id,
            source_id: input.id,
            tokens,
            features,
            metadata: BTreeMap::new(),
        })
    }

    fn normalize(&self, text: &str, config: &NormalizeConfig) -> String {
        let mut result = text.to_string();

        if config.lowercase {
            result = result.to_lowercase();
        }

        if config.trim {
            result = result.trim().to_string();
        }

        if config.collapse_whitespace {
            let mut collapsed = String::new();
            let mut prev_space = false;

            for c in result.chars() {
                if c.is_whitespace() {
                    if !prev_space {
                        collapsed.push(' ');
                        prev_space = true;
                    }
                } else {
                    collapsed.push(c);
                    prev_space = false;
                }
            }

            result = collapsed;
        }

        result
    }

    fn tokenize(&mut self, text: &str, config: &TokenizeConfig) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut line = 0;
        let mut column = 0;
        let mut offset = 0;

        match config.mode {
            TokenizeMode::Whitespace | TokenizeMode::Word => {
                for word in text.split_whitespace() {
                    let id = self.next_id.fetch_add(1, Ordering::Relaxed);

                    let token_text = if config.preserve_case {
                        word.to_string()
                    } else {
                        word.to_lowercase()
                    };

                    tokens.push(Token {
                        id,
                        token_type: self.classify_token(word),
                        text: token_text,
                        position: Position {
                            line,
                            column,
                            offset,
                            length: word.len(),
                        },
                        features: BTreeMap::new(),
                    });

                    offset += word.len() + 1;
                    column += word.len() + 1;
                }
            }
            TokenizeMode::Character => {
                for c in text.chars() {
                    if c.is_whitespace() {
                        if c == '\n' {
                            line += 1;
                            column = 0;
                        }
                        offset += 1;
                        column += 1;
                        continue;
                    }

                    let id = self.next_id.fetch_add(1, Ordering::Relaxed);

                    tokens.push(Token {
                        id,
                        token_type: TokenType::Unknown,
                        text: c.to_string(),
                        position: Position {
                            line,
                            column,
                            offset,
                            length: 1,
                        },
                        features: BTreeMap::new(),
                    });

                    offset += 1;
                    column += 1;
                }
            }
            TokenizeMode::CodeAware => {
                // Simple code tokenizer
                let mut current = String::new();
                let mut start_offset = 0;

                for (i, c) in text.chars().enumerate() {
                    if c.is_alphanumeric() || c == '_' {
                        if current.is_empty() {
                            start_offset = i;
                        }
                        current.push(c);
                    } else {
                        if !current.is_empty() {
                            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                            tokens.push(Token {
                                id,
                                token_type: self.classify_token(&current),
                                text: current.clone(),
                                position: Position {
                                    line: 0,
                                    column: start_offset,
                                    offset: start_offset,
                                    length: current.len(),
                                },
                                features: BTreeMap::new(),
                            });
                            current.clear();
                        }

                        if !c.is_whitespace() {
                            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                            tokens.push(Token {
                                id,
                                token_type: self.classify_operator(c),
                                text: c.to_string(),
                                position: Position {
                                    line: 0,
                                    column: i,
                                    offset: i,
                                    length: 1,
                                },
                                features: BTreeMap::new(),
                            });
                        }
                    }
                }

                // Last token
                if !current.is_empty() {
                    let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                    tokens.push(Token {
                        id,
                        token_type: self.classify_token(&current),
                        text: current,
                        position: Position {
                            line: 0,
                            column: start_offset,
                            offset: start_offset,
                            length: 0,
                        },
                        features: BTreeMap::new(),
                    });
                }
            }
        }

        // Limit tokens
        tokens.truncate(self.config.max_tokens);

        tokens
    }

    fn classify_token(&self, text: &str) -> TokenType {
        let keywords = ["fn", "let", "if", "else", "for", "while", "match", "return", "struct", "impl", "pub", "use"];

        if keywords.contains(&text) {
            TokenType::Keyword
        } else if text.chars().all(|c| c.is_numeric()) {
            TokenType::Literal
        } else if text.starts_with("//") || text.starts_with("/*") {
            TokenType::Comment
        } else {
            TokenType::Identifier
        }
    }

    fn classify_operator(&self, c: char) -> TokenType {
        match c {
            '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' => TokenType::Operator,
            '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':' | '.' => TokenType::Punctuation,
            _ => TokenType::Unknown,
        }
    }

    fn filter(&self, tokens: Vec<Token>, config: &FilterConfig) -> Vec<Token> {
        tokens.into_iter()
            .filter(|t| {
                // Min length
                if let Some(min) = config.min_length {
                    if t.text.len() < min {
                        return false;
                    }
                }

                // Max length
                if let Some(max) = config.max_length {
                    if t.text.len() > max {
                        return false;
                    }
                }

                // Exclude types
                if config.exclude_types.contains(&t.token_type) {
                    return false;
                }

                true
            })
            .collect()
    }

    fn extract(&self, tokens: &[Token], _config: &ExtractConfig) -> FeatureSet {
        let mut features = FeatureSet::default();

        // Basic features
        features.numeric.insert("token_count".into(), tokens.len() as f64);

        // Token type distribution
        let mut type_counts: BTreeMap<String, f64> = BTreeMap::new();
        for token in tokens {
            let type_name = format!("{:?}", token.token_type);
            *type_counts.entry(type_name).or_insert(0.0) += 1.0;
        }

        for (type_name, count) in type_counts {
            features.numeric.insert(format!("type_{}", type_name), count);
        }

        // Average token length
        if !tokens.is_empty() {
            let avg_len: f64 = tokens.iter().map(|t| t.text.len() as f64).sum::<f64>() / tokens.len() as f64;
            features.numeric.insert("avg_token_length".into(), avg_len);
        }

        features
    }

    fn transform(&self, features: &mut FeatureSet, config: &TransformConfig) {
        match &config.operation {
            TransformOp::Scale { min, max } => {
                if let Some(value) = features.numeric.get_mut(&config.target) {
                    *value = (*value - min) / (max - min);
                }
            }
            TransformOp::Log => {
                if let Some(value) = features.numeric.get_mut(&config.target) {
                    *value = (*value + 1.0).ln();
                }
            }
            TransformOp::ZScore => {
                // Would need mean/std
            }
            TransformOp::Embed { dimensions: _ } => {
                // Would create embedding
            }
        }
    }

    /// Get pipeline
    pub fn get_pipeline(&self, id: u64) -> Option<&Pipeline> {
        self.pipelines.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &PreprocessorStats {
        &self.stats
    }
}

impl Default for Preprocessor {
    fn default() -> Self {
        Self::new(PreprocessorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(text: &str) -> RawInput {
        RawInput {
            id: 1,
            source: "test".into(),
            content_type: ContentType::Text,
            data: InputData::Text(text.into()),
            timestamp: Timestamp::now(),
        }
    }

    #[test]
    fn test_create_pipeline() {
        let mut pp = Preprocessor::default();

        let id = pp.create_pipeline("basic", vec![
            PreprocessStep::Normalize(NormalizeConfig {
                lowercase: true,
                trim: true,
                collapse_whitespace: true,
                remove_accents: false,
            }),
            PreprocessStep::Tokenize(TokenizeConfig {
                mode: TokenizeMode::Word,
                preserve_case: false,
                include_positions: true,
            }),
        ]);

        assert!(pp.get_pipeline(id).is_some());
    }

    #[test]
    fn test_process_text() {
        let mut pp = Preprocessor::default();

        pp.create_pipeline("test", vec![
            PreprocessStep::Normalize(NormalizeConfig {
                lowercase: true,
                trim: true,
                collapse_whitespace: true,
                remove_accents: false,
            }),
            PreprocessStep::Tokenize(TokenizeConfig {
                mode: TokenizeMode::Word,
                preserve_case: false,
                include_positions: true,
            }),
        ]);

        let input = make_input("  Hello   World  ");
        let output = pp.process(input, None);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.tokens.len(), 2);
    }

    #[test]
    fn test_code_tokenize() {
        let mut pp = Preprocessor::default();

        pp.create_pipeline("code", vec![
            PreprocessStep::Tokenize(TokenizeConfig {
                mode: TokenizeMode::CodeAware,
                preserve_case: true,
                include_positions: true,
            }),
        ]);

        let input = make_input("fn main() { let x = 1; }");
        let output = pp.process(input, None).unwrap();

        assert!(output.tokens.iter().any(|t| t.token_type == TokenType::Keyword));
        assert!(output.tokens.iter().any(|t| t.token_type == TokenType::Identifier));
    }

    #[test]
    fn test_filter() {
        let mut pp = Preprocessor::default();

        pp.create_pipeline("filtered", vec![
            PreprocessStep::Tokenize(TokenizeConfig {
                mode: TokenizeMode::Word,
                preserve_case: true,
                include_positions: true,
            }),
            PreprocessStep::Filter(FilterConfig {
                min_length: Some(3),
                max_length: None,
                exclude_types: vec![],
                exclude_patterns: vec![],
            }),
        ]);

        let input = make_input("a to the big world");
        let output = pp.process(input, None).unwrap();

        // "a" and "to" filtered out
        assert_eq!(output.tokens.len(), 3);
    }

    #[test]
    fn test_extract_features() {
        let mut pp = Preprocessor::default();

        pp.create_pipeline("features", vec![
            PreprocessStep::Tokenize(TokenizeConfig {
                mode: TokenizeMode::Word,
                preserve_case: true,
                include_positions: true,
            }),
            PreprocessStep::Extract(ExtractConfig {
                features: vec!["count".into()],
                n_grams: None,
                tf_idf: false,
            }),
        ]);

        let input = make_input("one two three four five");
        let output = pp.process(input, None).unwrap();

        assert_eq!(output.features.numeric.get("token_count"), Some(&5.0));
    }
}
