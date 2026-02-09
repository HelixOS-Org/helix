//! # Cognitive Factory
//!
//! Factory for creating cognitive components and pipelines.
//! Provides templating and configuration-driven instantiation.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, ComponentId, Timestamp};

// ============================================================================
// FACTORY TYPES
// ============================================================================

/// Component template
#[derive(Debug, Clone)]
pub struct ComponentTemplate {
    /// Template ID
    pub id: u64,
    /// Template name
    pub name: String,
    /// Component type
    pub component_type: ComponentType,
    /// Base configuration
    pub config: ComponentConfig,
    /// Parameters
    pub parameters: Vec<TemplateParameter>,
    /// Defaults
    pub defaults: BTreeMap<String, ConfigValue>,
    /// Description
    pub description: String,
    /// Tags
    pub tags: Vec<String>,
}

/// Component type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Processing unit
    Processor,
    /// Storage unit
    Storage,
    /// Connector/adapter
    Connector,
    /// Pipeline
    Pipeline,
    /// Model wrapper
    Model,
    /// Monitor
    Monitor,
    /// Router
    Router,
    /// Aggregator
    Aggregator,
}

/// Component configuration
#[derive(Debug, Clone)]
pub struct ComponentConfig {
    /// Configuration values
    pub values: BTreeMap<String, ConfigValue>,
    /// Resource limits
    pub resources: ResourceLimits,
    /// Dependencies
    pub dependencies: Vec<String>,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            resources: ResourceLimits::default(),
            dependencies: Vec::new(),
        }
    }
}

/// Configuration value
#[derive(Debug, Clone)]
pub enum ConfigValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<ConfigValue>),
    Object(BTreeMap<String, ConfigValue>),
}

impl ConfigValue {
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    #[inline]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

/// Resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Memory limit (bytes)
    pub memory_bytes: u64,
    /// CPU limit (percentage)
    pub cpu_percent: u32,
    /// Max concurrent operations
    pub max_concurrent: u32,
    /// Timeout (ns)
    pub timeout_ns: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 64 * 1024 * 1024, // 64 MB
            cpu_percent: 100,
            max_concurrent: 100,
            timeout_ns: 30_000_000_000, // 30 seconds
        }
    }
}

/// Template parameter
#[derive(Debug, Clone)]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: ParameterType,
    /// Required
    pub required: bool,
    /// Description
    pub description: String,
    /// Validation
    pub validation: Option<ParameterValidation>,
}

/// Parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    Bool,
    Int,
    Float,
    String,
    Array,
    Object,
}

/// Parameter validation
#[derive(Debug, Clone)]
pub enum ParameterValidation {
    /// Range for numbers
    Range { min: f64, max: f64 },
    /// Pattern for strings
    Pattern(String),
    /// Enum values
    Enum(Vec<String>),
    /// Custom validator name
    Custom(String),
}

// ============================================================================
// CREATED INSTANCES
// ============================================================================

/// Created component instance
#[derive(Debug)]
pub struct ComponentInstance {
    /// Instance ID
    pub id: ComponentId,
    /// Template used
    pub template_id: u64,
    /// Owner domain
    pub owner: DomainId,
    /// Configuration
    pub config: ComponentConfig,
    /// Status
    pub status: InstanceStatus,
    /// Created at
    pub created_at: Timestamp,
    /// Started at
    pub started_at: Option<Timestamp>,
}

/// Instance status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    Created,
    Initializing,
    Running,
    Paused,
    Stopping,
    Stopped,
    Failed,
}

// ============================================================================
// PIPELINE BUILDER
// ============================================================================

/// Pipeline definition
#[derive(Debug, Clone)]
pub struct PipelineDefinition {
    /// Pipeline ID
    pub id: u64,
    /// Pipeline name
    pub name: String,
    /// Stages
    pub stages: Vec<PipelineStage>,
    /// Error handling
    pub error_handling: ErrorHandling,
    /// Parallelism
    pub parallelism: u32,
}

/// Pipeline stage
#[derive(Debug, Clone)]
pub struct PipelineStage {
    /// Stage name
    pub name: String,
    /// Component template
    pub template: String,
    /// Configuration overrides
    pub config: BTreeMap<String, ConfigValue>,
    /// Inputs
    pub inputs: Vec<StageInput>,
    /// Outputs
    pub outputs: Vec<StageOutput>,
}

/// Stage input
#[derive(Debug, Clone)]
pub struct StageInput {
    /// Input name
    pub name: String,
    /// Source stage
    pub source: String,
    /// Source output
    pub output: String,
}

/// Stage output
#[derive(Debug, Clone)]
pub struct StageOutput {
    /// Output name
    pub name: String,
    /// Output type
    pub output_type: String,
}

/// Error handling
#[derive(Debug, Clone)]
pub enum ErrorHandling {
    /// Stop on first error
    Fail,
    /// Continue on error
    Continue,
    /// Retry with backoff
    Retry { max_retries: u32, backoff_ns: u64 },
    /// Skip failed items
    Skip,
    /// Custom handler
    Custom(String),
}

/// Pipeline builder
pub struct PipelineBuilder {
    name: String,
    stages: Vec<PipelineStage>,
    error_handling: ErrorHandling,
    parallelism: u32,
}

impl PipelineBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            stages: Vec::new(),
            error_handling: ErrorHandling::Fail,
            parallelism: 1,
        }
    }

    #[inline]
    pub fn add_stage(
        mut self,
        name: &str,
        template: &str,
        config: BTreeMap<String, ConfigValue>,
    ) -> Self {
        self.stages.push(PipelineStage {
            name: name.into(),
            template: template.into(),
            config,
            inputs: Vec::new(),
            outputs: Vec::new(),
        });
        self
    }

    #[inline]
    pub fn connect(mut self, from_stage: &str, from_output: &str, to_stage: &str, to_input: &str) -> Self {
        if let Some(stage) = self.stages.iter_mut().find(|s| s.name == to_stage) {
            stage.inputs.push(StageInput {
                name: to_input.into(),
                source: from_stage.into(),
                output: from_output.into(),
            });
        }
        self
    }

    #[inline(always)]
    pub fn with_error_handling(mut self, handling: ErrorHandling) -> Self {
        self.error_handling = handling;
        self
    }

    #[inline(always)]
    pub fn with_parallelism(mut self, parallelism: u32) -> Self {
        self.parallelism = parallelism;
        self
    }

    #[inline]
    pub fn build(self, id: u64) -> PipelineDefinition {
        PipelineDefinition {
            id,
            name: self.name,
            stages: self.stages,
            error_handling: self.error_handling,
            parallelism: self.parallelism,
        }
    }
}

// ============================================================================
// FACTORY
// ============================================================================

/// Cognitive factory
pub struct CognitiveFactory {
    /// Templates
    templates: BTreeMap<u64, ComponentTemplate>,
    /// Templates by name
    templates_by_name: BTreeMap<String, u64>,
    /// Instances
    instances: BTreeMap<ComponentId, ComponentInstance>,
    /// Pipelines
    pipelines: BTreeMap<u64, PipelineDefinition>,
    /// Next template ID
    next_template_id: AtomicU64,
    /// Next pipeline ID
    next_pipeline_id: AtomicU64,
    /// Next instance ID
    next_instance_id: AtomicU64,
    /// Configuration
    config: FactoryConfig,
    /// Statistics
    stats: FactoryStats,
}

/// Factory configuration
#[derive(Debug, Clone)]
pub struct FactoryConfig {
    /// Maximum templates
    pub max_templates: usize,
    /// Maximum instances
    pub max_instances: usize,
    /// Maximum pipelines
    pub max_pipelines: usize,
    /// Default resource limits
    pub default_limits: ResourceLimits,
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            max_templates: 1000,
            max_instances: 10000,
            max_pipelines: 100,
            default_limits: ResourceLimits::default(),
        }
    }
}

/// Factory statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FactoryStats {
    /// Templates registered
    pub templates: u64,
    /// Instances created
    pub instances_created: u64,
    /// Instances running
    pub instances_running: u64,
    /// Pipelines defined
    pub pipelines: u64,
}

impl CognitiveFactory {
    /// Create new factory
    pub fn new(config: FactoryConfig) -> Self {
        Self {
            templates: BTreeMap::new(),
            templates_by_name: BTreeMap::new(),
            instances: BTreeMap::new(),
            pipelines: BTreeMap::new(),
            next_template_id: AtomicU64::new(1),
            next_pipeline_id: AtomicU64::new(1),
            next_instance_id: AtomicU64::new(1),
            config,
            stats: FactoryStats::default(),
        }
    }

    /// Register template
    pub fn register_template(
        &mut self,
        name: &str,
        component_type: ComponentType,
        config: ComponentConfig,
        parameters: Vec<TemplateParameter>,
        description: &str,
    ) -> Result<u64, &'static str> {
        if self.templates.len() >= self.config.max_templates {
            return Err("Template limit exceeded");
        }

        if self.templates_by_name.contains_key(name) {
            return Err("Template name already exists");
        }

        let id = self.next_template_id.fetch_add(1, Ordering::Relaxed);

        let template = ComponentTemplate {
            id,
            name: name.into(),
            component_type,
            config,
            parameters,
            defaults: BTreeMap::new(),
            description: description.into(),
            tags: Vec::new(),
        };

        self.templates.insert(id, template);
        self.templates_by_name.insert(name.into(), id);
        self.stats.templates += 1;

        Ok(id)
    }

    /// Get template
    #[inline(always)]
    pub fn get_template(&self, id: u64) -> Option<&ComponentTemplate> {
        self.templates.get(&id)
    }

    /// Get template by name
    #[inline(always)]
    pub fn get_template_by_name(&self, name: &str) -> Option<&ComponentTemplate> {
        let id = self.templates_by_name.get(name)?;
        self.templates.get(id)
    }

    /// Create instance from template
    pub fn create(
        &mut self,
        template_id: u64,
        owner: DomainId,
        config_overrides: BTreeMap<String, ConfigValue>,
    ) -> Result<ComponentId, &'static str> {
        if self.instances.len() >= self.config.max_instances {
            return Err("Instance limit exceeded");
        }

        let template = self.templates.get(&template_id)
            .ok_or("Template not found")?;

        // Merge configuration
        let mut config = template.config.clone();
        for (key, value) in config_overrides {
            config.values.insert(key, value);
        }

        // Validate required parameters
        for param in &template.parameters {
            if param.required && !config.values.contains_key(&param.name) {
                if let Some(default) = template.defaults.get(&param.name) {
                    config.values.insert(param.name.clone(), default.clone());
                } else {
                    return Err("Missing required parameter");
                }
            }
        }

        let id = ComponentId::new(self.next_instance_id.fetch_add(1, Ordering::Relaxed));

        let instance = ComponentInstance {
            id,
            template_id,
            owner,
            config,
            status: InstanceStatus::Created,
            created_at: Timestamp::now(),
            started_at: None,
        };

        self.instances.insert(id, instance);
        self.stats.instances_created += 1;

        Ok(id)
    }

    /// Start instance
    pub fn start(&mut self, id: ComponentId) -> Result<(), &'static str> {
        let instance = self.instances.get_mut(&id)
            .ok_or("Instance not found")?;

        if instance.status != InstanceStatus::Created && instance.status != InstanceStatus::Stopped {
            return Err("Instance cannot be started from current state");
        }

        instance.status = InstanceStatus::Running;
        instance.started_at = Some(Timestamp::now());
        self.stats.instances_running += 1;

        Ok(())
    }

    /// Stop instance
    pub fn stop(&mut self, id: ComponentId) -> Result<(), &'static str> {
        let instance = self.instances.get_mut(&id)
            .ok_or("Instance not found")?;

        if instance.status != InstanceStatus::Running {
            return Err("Instance is not running");
        }

        instance.status = InstanceStatus::Stopped;
        self.stats.instances_running = self.stats.instances_running.saturating_sub(1);

        Ok(())
    }

    /// Destroy instance
    #[inline]
    pub fn destroy(&mut self, id: ComponentId) -> Result<(), &'static str> {
        let instance = self.instances.remove(&id)
            .ok_or("Instance not found")?;

        if instance.status == InstanceStatus::Running {
            self.stats.instances_running = self.stats.instances_running.saturating_sub(1);
        }

        Ok(())
    }

    /// Get instance
    #[inline(always)]
    pub fn get_instance(&self, id: ComponentId) -> Option<&ComponentInstance> {
        self.instances.get(&id)
    }

    /// Create pipeline
    pub fn create_pipeline(&mut self, builder: PipelineBuilder) -> Result<u64, &'static str> {
        if self.pipelines.len() >= self.config.max_pipelines {
            return Err("Pipeline limit exceeded");
        }

        // Validate stages reference valid templates
        for stage in &builder.stages {
            if !self.templates_by_name.contains_key(&stage.template) {
                return Err("Unknown template in pipeline");
            }
        }

        let id = self.next_pipeline_id.fetch_add(1, Ordering::Relaxed);
        let pipeline = builder.build(id);

        self.pipelines.insert(id, pipeline);
        self.stats.pipelines += 1;

        Ok(id)
    }

    /// Get pipeline
    #[inline(always)]
    pub fn get_pipeline(&self, id: u64) -> Option<&PipelineDefinition> {
        self.pipelines.get(&id)
    }

    /// Instantiate pipeline
    pub fn instantiate_pipeline(
        &mut self,
        pipeline_id: u64,
        owner: DomainId,
    ) -> Result<Vec<ComponentId>, &'static str> {
        let pipeline = self.pipelines.get(&pipeline_id)
            .ok_or("Pipeline not found")?
            .clone();

        let mut instances = Vec::new();

        for stage in &pipeline.stages {
            let template_id = self.templates_by_name.get(&stage.template)
                .ok_or("Template not found")?;

            let instance_id = self.create(*template_id, owner, stage.config.clone())?;
            instances.push(instance_id);
        }

        Ok(instances)
    }

    /// List templates by type
    #[inline]
    pub fn templates_by_type(&self, component_type: ComponentType) -> Vec<&ComponentTemplate> {
        self.templates.values()
            .filter(|t| t.component_type == component_type)
            .collect()
    }

    /// List running instances
    #[inline]
    pub fn running_instances(&self) -> Vec<&ComponentInstance> {
        self.instances.values()
            .filter(|i| i.status == InstanceStatus::Running)
            .collect()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &FactoryStats {
        &self.stats
    }
}

impl Default for CognitiveFactory {
    fn default() -> Self {
        Self::new(FactoryConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_registration() {
        let mut factory = CognitiveFactory::default();

        let id = factory.register_template(
            "test_processor",
            ComponentType::Processor,
            ComponentConfig::default(),
            vec![],
            "Test processor",
        ).unwrap();

        assert!(factory.get_template(id).is_some());
        assert!(factory.get_template_by_name("test_processor").is_some());
    }

    #[test]
    fn test_instance_creation() {
        let mut factory = CognitiveFactory::default();
        let owner = DomainId::new(1);

        let template_id = factory.register_template(
            "test",
            ComponentType::Processor,
            ComponentConfig::default(),
            vec![],
            "Test",
        ).unwrap();

        let instance_id = factory.create(template_id, owner, BTreeMap::new()).unwrap();

        let instance = factory.get_instance(instance_id).unwrap();
        assert_eq!(instance.status, InstanceStatus::Created);

        factory.start(instance_id).unwrap();
        assert_eq!(factory.get_instance(instance_id).unwrap().status, InstanceStatus::Running);

        factory.stop(instance_id).unwrap();
        assert_eq!(factory.get_instance(instance_id).unwrap().status, InstanceStatus::Stopped);
    }

    #[test]
    fn test_pipeline_builder() {
        let mut factory = CognitiveFactory::default();

        factory.register_template("input", ComponentType::Connector, ComponentConfig::default(), vec![], "Input").unwrap();
        factory.register_template("process", ComponentType::Processor, ComponentConfig::default(), vec![], "Process").unwrap();
        factory.register_template("output", ComponentType::Connector, ComponentConfig::default(), vec![], "Output").unwrap();

        let builder = PipelineBuilder::new("test_pipeline")
            .add_stage("stage1", "input", BTreeMap::new())
            .add_stage("stage2", "process", BTreeMap::new())
            .add_stage("stage3", "output", BTreeMap::new())
            .connect("stage1", "data", "stage2", "input")
            .connect("stage2", "result", "stage3", "input")
            .with_parallelism(2);

        let pipeline_id = factory.create_pipeline(builder).unwrap();
        let pipeline = factory.get_pipeline(pipeline_id).unwrap();

        assert_eq!(pipeline.stages.len(), 3);
        assert_eq!(pipeline.parallelism, 2);
    }

    #[test]
    fn test_required_parameters() {
        let mut factory = CognitiveFactory::default();
        let owner = DomainId::new(1);

        let template_id = factory.register_template(
            "test",
            ComponentType::Processor,
            ComponentConfig::default(),
            vec![TemplateParameter {
                name: "required_param".into(),
                param_type: ParameterType::String,
                required: true,
                description: "Required".into(),
                validation: None,
            }],
            "Test",
        ).unwrap();

        // Should fail - missing required parameter
        assert!(factory.create(template_id, owner, BTreeMap::new()).is_err());

        // Should succeed with parameter
        let mut config = BTreeMap::new();
        config.insert("required_param".into(), ConfigValue::String("value".into()));
        assert!(factory.create(template_id, owner, config).is_ok());
    }
}
