//! # Cognitive Output Manager
//!
//! Output formatting and delivery for cognitive operations.
//! Handles result aggregation, formatting, and routing.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// OUTPUT TYPES
// ============================================================================

/// Cognitive output
#[derive(Debug, Clone)]
pub struct CognitiveOutput {
    /// Output ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Output type
    pub output_type: OutputType,
    /// Content
    pub content: OutputContent,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Priority
    pub priority: OutputPriority,
    /// Tags
    pub tags: Vec<String>,
}

/// Output type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// Decision output
    Decision,
    /// Action output
    Action,
    /// Insight output
    Insight,
    /// Prediction output
    Prediction,
    /// Analysis output
    Analysis,
    /// Status update
    Status,
    /// Alert
    Alert,
    /// Log entry
    Log,
    /// Metric
    Metric,
}

/// Output content
#[derive(Debug, Clone)]
pub enum OutputContent {
    /// Text content
    Text(String),
    /// Structured data
    Structured(BTreeMap<String, OutputValue>),
    /// Binary data
    Binary(Vec<u8>),
    /// Multiple outputs
    Multiple(Vec<OutputContent>),
    /// Reference to another output
    Reference(u64),
}

/// Output value
#[derive(Debug, Clone)]
pub enum OutputValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<OutputValue>),
    Object(BTreeMap<String, OutputValue>),
}

/// Output priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputPriority {
    Low,
    Normal,
    High,
    Critical,
}

// ============================================================================
// OUTPUT FORMATTING
// ============================================================================

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Plain text
    Text,
    /// JSON
    Json,
    /// Markdown
    Markdown,
    /// Binary
    Binary,
    /// Human readable
    Human,
}

/// Output formatter
pub struct OutputFormatter {
    /// Format
    format: OutputFormat,
    /// Options
    options: FormatterOptions,
}

/// Formatter options
#[derive(Debug, Clone)]
pub struct FormatterOptions {
    /// Pretty print
    pub pretty: bool,
    /// Include metadata
    pub include_metadata: bool,
    /// Include timestamp
    pub include_timestamp: bool,
    /// Max length (0 = unlimited)
    pub max_length: usize,
    /// Truncation suffix
    pub truncation_suffix: String,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        Self {
            pretty: true,
            include_metadata: true,
            include_timestamp: true,
            max_length: 0,
            truncation_suffix: "...".into(),
        }
    }
}

impl OutputFormatter {
    /// Create new formatter
    pub fn new(format: OutputFormat, options: FormatterOptions) -> Self {
        Self { format, options }
    }

    /// Format output
    pub fn format(&self, output: &CognitiveOutput) -> String {
        match self.format {
            OutputFormat::Text => self.format_text(output),
            OutputFormat::Json => self.format_json(output),
            OutputFormat::Markdown => self.format_markdown(output),
            OutputFormat::Human => self.format_human(output),
            OutputFormat::Binary => self.format_binary(output),
        }
    }

    fn format_text(&self, output: &CognitiveOutput) -> String {
        let mut result = String::new();

        if self.options.include_timestamp {
            result.push_str(&format!("[{}] ", output.timestamp.raw()));
        }

        result.push_str(&format!("[{:?}] ", output.output_type));

        match &output.content {
            OutputContent::Text(text) => result.push_str(text),
            OutputContent::Structured(data) => {
                result.push_str(&self.format_structured(data));
            },
            OutputContent::Binary(data) => {
                result.push_str(&format!("<binary {} bytes>", data.len()));
            },
            OutputContent::Multiple(outputs) => {
                for (i, content) in outputs.iter().enumerate() {
                    if i > 0 {
                        result.push_str(" | ");
                    }
                    result.push_str(&self.format_content(content));
                }
            },
            OutputContent::Reference(id) => {
                result.push_str(&format!("<ref:{}>", id));
            },
        }

        self.truncate(result)
    }

    fn format_json(&self, output: &CognitiveOutput) -> String {
        let mut result = String::from("{");

        result.push_str(&format!("\"id\":{},", output.id));
        result.push_str(&format!("\"type\":\"{:?}\",", output.output_type));
        result.push_str(&format!("\"source\":{},", output.source.raw()));

        if self.options.include_timestamp {
            result.push_str(&format!("\"timestamp\":{},", output.timestamp.raw()));
        }

        result.push_str("\"content\":");
        result.push_str(&self.format_content_json(&output.content));

        if self.options.include_metadata && !output.metadata.is_empty() {
            result.push_str(",\"metadata\":{");
            let pairs: Vec<_> = output
                .metadata
                .iter()
                .map(|(k, v)| format!("\"{}\":\"{}\"", k, v))
                .collect();
            result.push_str(&pairs.join(","));
            result.push('}');
        }

        result.push('}');
        result
    }

    fn format_content_json(&self, content: &OutputContent) -> String {
        match content {
            OutputContent::Text(text) => format!("\"{}\"", text),
            OutputContent::Structured(data) => {
                let mut result = String::from("{");
                let pairs: Vec<_> = data
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, self.format_value_json(v)))
                    .collect();
                result.push_str(&pairs.join(","));
                result.push('}');
                result
            },
            OutputContent::Binary(data) => {
                format!("\"<binary {} bytes>\"", data.len())
            },
            OutputContent::Multiple(outputs) => {
                let items: Vec<_> = outputs
                    .iter()
                    .map(|c| self.format_content_json(c))
                    .collect();
                format!("[{}]", items.join(","))
            },
            OutputContent::Reference(id) => format!("{{\"$ref\":{}}}", id),
        }
    }

    fn format_value_json(&self, value: &OutputValue) -> String {
        match value {
            OutputValue::Null => "null".into(),
            OutputValue::Bool(b) => format!("{}", b),
            OutputValue::Int(i) => format!("{}", i),
            OutputValue::Float(f) => format!("{}", f),
            OutputValue::String(s) => format!("\"{}\"", s),
            OutputValue::Array(arr) => {
                let items: Vec<_> = arr.iter().map(|v| self.format_value_json(v)).collect();
                format!("[{}]", items.join(","))
            },
            OutputValue::Object(obj) => {
                let pairs: Vec<_> = obj
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, self.format_value_json(v)))
                    .collect();
                format!("{{{}}}", pairs.join(","))
            },
        }
    }

    fn format_markdown(&self, output: &CognitiveOutput) -> String {
        let mut result = String::new();

        result.push_str(&format!("## {:?} Output\n\n", output.output_type));

        if self.options.include_timestamp {
            result.push_str(&format!("*Timestamp: {}*\n\n", output.timestamp.raw()));
        }

        match &output.content {
            OutputContent::Text(text) => {
                result.push_str(text);
                result.push('\n');
            },
            OutputContent::Structured(data) => {
                result.push_str("| Key | Value |\n");
                result.push_str("|-----|-------|\n");
                for (k, v) in data {
                    result.push_str(&format!("| {} | {} |\n", k, self.format_value_simple(v)));
                }
            },
            _ => {
                result.push_str(&self.format_content(&output.content));
            },
        }

        if !output.tags.is_empty() {
            result.push_str("\n**Tags:** ");
            result.push_str(&output.tags.join(", "));
            result.push('\n');
        }

        result
    }

    fn format_human(&self, output: &CognitiveOutput) -> String {
        let mut result = String::new();

        let type_name = match output.output_type {
            OutputType::Decision => "Decision",
            OutputType::Action => "Action",
            OutputType::Insight => "Insight",
            OutputType::Prediction => "Prediction",
            OutputType::Analysis => "Analysis",
            OutputType::Status => "Status",
            OutputType::Alert => "Alert",
            OutputType::Log => "Log",
            OutputType::Metric => "Metric",
        };

        result.push_str(&format!("[{}] ", type_name));

        match &output.content {
            OutputContent::Text(text) => result.push_str(text),
            OutputContent::Structured(data) => {
                let items: Vec<_> = data
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.format_value_simple(v)))
                    .collect();
                result.push_str(&items.join(", "));
            },
            _ => result.push_str(&self.format_content(&output.content)),
        }

        self.truncate(result)
    }

    fn format_binary(&self, output: &CognitiveOutput) -> String {
        match &output.content {
            OutputContent::Binary(data) => {
                // Hex dump
                data.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ")
            },
            _ => self.format_text(output),
        }
    }

    fn format_content(&self, content: &OutputContent) -> String {
        match content {
            OutputContent::Text(text) => text.clone(),
            OutputContent::Structured(data) => self.format_structured(data),
            OutputContent::Binary(data) => format!("<binary {} bytes>", data.len()),
            OutputContent::Multiple(outputs) => outputs
                .iter()
                .map(|c| self.format_content(c))
                .collect::<Vec<_>>()
                .join(", "),
            OutputContent::Reference(id) => format!("<ref:{}>", id),
        }
    }

    fn format_structured(&self, data: &BTreeMap<String, OutputValue>) -> String {
        data.iter()
            .map(|(k, v)| format!("{}={}", k, self.format_value_simple(v)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn format_value_simple(&self, value: &OutputValue) -> String {
        match value {
            OutputValue::Null => "null".into(),
            OutputValue::Bool(b) => format!("{}", b),
            OutputValue::Int(i) => format!("{}", i),
            OutputValue::Float(f) => format!("{:.2}", f),
            OutputValue::String(s) => s.clone(),
            OutputValue::Array(arr) => {
                format!("[{} items]", arr.len())
            },
            OutputValue::Object(obj) => {
                format!("{{{} fields}}", obj.len())
            },
        }
    }

    fn truncate(&self, mut s: String) -> String {
        if self.options.max_length > 0 && s.len() > self.options.max_length {
            s.truncate(self.options.max_length - self.options.truncation_suffix.len());
            s.push_str(&self.options.truncation_suffix);
        }
        s
    }
}

// ============================================================================
// OUTPUT MANAGER
// ============================================================================

/// Output manager
pub struct OutputManager {
    /// Outputs
    outputs: Vec<CognitiveOutput>,
    /// Outputs by type
    by_type: BTreeMap<OutputType, Vec<u64>>,
    /// Outputs by source
    by_source: BTreeMap<DomainId, Vec<u64>>,
    /// Subscribers
    subscribers: BTreeMap<u64, OutputSubscription>,
    /// Next output ID
    next_output_id: AtomicU64,
    /// Next subscription ID
    next_sub_id: AtomicU64,
    /// Formatters
    formatters: BTreeMap<OutputFormat, OutputFormatter>,
    /// Configuration
    config: OutputConfig,
    /// Statistics
    stats: OutputStats,
}

/// Output subscription
#[derive(Debug, Clone)]
pub struct OutputSubscription {
    /// Subscription ID
    pub id: u64,
    /// Subscriber domain
    pub subscriber: DomainId,
    /// Filter by types
    pub types: Option<Vec<OutputType>>,
    /// Filter by sources
    pub sources: Option<Vec<DomainId>>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Minimum priority
    pub min_priority: OutputPriority,
}

/// Output configuration
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Maximum outputs to keep
    pub max_outputs: usize,
    /// Default format
    pub default_format: OutputFormat,
    /// Enable aggregation
    pub enable_aggregation: bool,
    /// Aggregation window (ns)
    pub aggregation_window_ns: u64,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_outputs: 10000,
            default_format: OutputFormat::Human,
            enable_aggregation: true,
            aggregation_window_ns: 1_000_000_000, // 1 second
        }
    }
}

/// Output statistics
#[derive(Debug, Clone, Default)]
pub struct OutputStats {
    /// Total outputs
    pub total_outputs: u64,
    /// Outputs by type
    pub by_type: BTreeMap<OutputType, u64>,
    /// Deliveries
    pub deliveries: u64,
}

impl OutputManager {
    /// Create new output manager
    pub fn new(config: OutputConfig) -> Self {
        let mut formatters = BTreeMap::new();
        formatters.insert(
            OutputFormat::Text,
            OutputFormatter::new(OutputFormat::Text, FormatterOptions::default()),
        );
        formatters.insert(
            OutputFormat::Json,
            OutputFormatter::new(OutputFormat::Json, FormatterOptions::default()),
        );
        formatters.insert(
            OutputFormat::Markdown,
            OutputFormatter::new(OutputFormat::Markdown, FormatterOptions::default()),
        );
        formatters.insert(
            OutputFormat::Human,
            OutputFormatter::new(OutputFormat::Human, FormatterOptions::default()),
        );

        Self {
            outputs: Vec::new(),
            by_type: BTreeMap::new(),
            by_source: BTreeMap::new(),
            subscribers: BTreeMap::new(),
            next_output_id: AtomicU64::new(1),
            next_sub_id: AtomicU64::new(1),
            formatters,
            config,
            stats: OutputStats::default(),
        }
    }

    /// Emit output
    pub fn emit(
        &mut self,
        source: DomainId,
        output_type: OutputType,
        content: OutputContent,
        priority: OutputPriority,
        tags: Vec<String>,
    ) -> u64 {
        let id = self.next_output_id.fetch_add(1, Ordering::Relaxed);

        let output = CognitiveOutput {
            id,
            source,
            output_type,
            content,
            metadata: BTreeMap::new(),
            timestamp: Timestamp::now(),
            priority,
            tags,
        };

        // Index
        self.by_type
            .entry(output_type)
            .or_insert_with(Vec::new)
            .push(id);
        self.by_source
            .entry(source)
            .or_insert_with(Vec::new)
            .push(id);

        // Update stats
        self.stats.total_outputs += 1;
        *self.stats.by_type.entry(output_type).or_insert(0) += 1;

        // Store
        self.outputs.push(output);

        // Limit storage
        while self.outputs.len() > self.config.max_outputs {
            let removed = self.outputs.remove(0);
            if let Some(ids) = self.by_type.get_mut(&removed.output_type) {
                ids.retain(|&i| i != removed.id);
            }
            if let Some(ids) = self.by_source.get_mut(&removed.source) {
                ids.retain(|&i| i != removed.id);
            }
        }

        id
    }

    /// Subscribe to outputs
    pub fn subscribe(
        &mut self,
        subscriber: DomainId,
        types: Option<Vec<OutputType>>,
        sources: Option<Vec<DomainId>>,
        tags: Option<Vec<String>>,
        min_priority: OutputPriority,
    ) -> u64 {
        let id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);

        let subscription = OutputSubscription {
            id,
            subscriber,
            types,
            sources,
            tags,
            min_priority,
        };

        self.subscribers.insert(id, subscription);
        id
    }

    /// Unsubscribe
    pub fn unsubscribe(&mut self, id: u64) {
        self.subscribers.remove(&id);
    }

    /// Get outputs for subscriber
    pub fn poll(&self, subscription_id: u64) -> Vec<&CognitiveOutput> {
        let sub = match self.subscribers.get(&subscription_id) {
            Some(s) => s,
            None => return vec![],
        };

        self.outputs
            .iter()
            .filter(|o| {
                // Check priority
                if o.priority < sub.min_priority {
                    return false;
                }

                // Check types
                if let Some(types) = &sub.types {
                    if !types.contains(&o.output_type) {
                        return false;
                    }
                }

                // Check sources
                if let Some(sources) = &sub.sources {
                    if !sources.contains(&o.source) {
                        return false;
                    }
                }

                // Check tags
                if let Some(tags) = &sub.tags {
                    if !tags.iter().any(|t| o.tags.contains(t)) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Format output
    pub fn format(&self, output: &CognitiveOutput, format: OutputFormat) -> String {
        self.formatters
            .get(&format)
            .map(|f| f.format(output))
            .unwrap_or_else(|| format!("{:?}", output))
    }

    /// Get output by ID
    pub fn get(&self, id: u64) -> Option<&CognitiveOutput> {
        self.outputs.iter().find(|o| o.id == id)
    }

    /// Get outputs by type
    pub fn by_type(&self, output_type: OutputType) -> Vec<&CognitiveOutput> {
        self.outputs
            .iter()
            .filter(|o| o.output_type == output_type)
            .collect()
    }

    /// Get outputs by source
    pub fn by_source(&self, source: DomainId) -> Vec<&CognitiveOutput> {
        self.outputs.iter().filter(|o| o.source == source).collect()
    }

    /// Get recent outputs
    pub fn recent(&self, count: usize) -> &[CognitiveOutput] {
        let start = self.outputs.len().saturating_sub(count);
        &self.outputs[start..]
    }

    /// Get statistics
    pub fn stats(&self) -> &OutputStats {
        &self.stats
    }
}

impl Default for OutputManager {
    fn default() -> Self {
        Self::new(OutputConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_emission() {
        let mut manager = OutputManager::default();
        let domain = DomainId::new(1);

        let id = manager.emit(
            domain,
            OutputType::Decision,
            OutputContent::Text("Test decision".into()),
            OutputPriority::Normal,
            vec!["test".into()],
        );

        assert!(manager.get(id).is_some());
        assert_eq!(manager.stats().total_outputs, 1);
    }

    #[test]
    fn test_formatter() {
        let formatter = OutputFormatter::new(OutputFormat::Human, FormatterOptions::default());

        let output = CognitiveOutput {
            id: 1,
            source: DomainId::new(1),
            output_type: OutputType::Insight,
            content: OutputContent::Text("Important insight".into()),
            metadata: BTreeMap::new(),
            timestamp: Timestamp::now(),
            priority: OutputPriority::High,
            tags: vec![],
        };

        let formatted = formatter.format(&output);
        assert!(formatted.contains("Insight"));
        assert!(formatted.contains("Important insight"));
    }

    #[test]
    fn test_subscription() {
        let mut manager = OutputManager::default();
        let domain = DomainId::new(1);
        let subscriber = DomainId::new(2);

        manager.emit(
            domain,
            OutputType::Alert,
            OutputContent::Text("Alert!".into()),
            OutputPriority::High,
            vec![],
        );
        manager.emit(
            domain,
            OutputType::Log,
            OutputContent::Text("Log".into()),
            OutputPriority::Low,
            vec![],
        );

        let sub_id = manager.subscribe(
            subscriber,
            Some(vec![OutputType::Alert]),
            None,
            None,
            OutputPriority::Normal,
        );

        let outputs = manager.poll(sub_id);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].output_type, OutputType::Alert);
    }

    #[test]
    fn test_json_format() {
        let formatter = OutputFormatter::new(OutputFormat::Json, FormatterOptions::default());

        let mut data = BTreeMap::new();
        data.insert("key".into(), OutputValue::String("value".into()));

        let output = CognitiveOutput {
            id: 1,
            source: DomainId::new(1),
            output_type: OutputType::Analysis,
            content: OutputContent::Structured(data),
            metadata: BTreeMap::new(),
            timestamp: Timestamp::now(),
            priority: OutputPriority::Normal,
            tags: vec![],
        };

        let formatted = formatter.format(&output);
        assert!(formatted.contains("\"id\":1"));
        assert!(formatted.contains("\"key\":\"value\""));
    }
}
