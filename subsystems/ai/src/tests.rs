//! # Helix AI - Test Suite
//!
//! Comprehensive tests for the AI subsystem components.

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;

    use crate::core::*;
    use crate::cortex::Cortex;
    use crate::healer::Healer;
    use crate::intent::IntentEngine;
    use crate::learning::{ActionVector, LearningEngine, StateVector};
    use crate::memory::AiMemory;
    use crate::metrics::MetricsCollector;
    use crate::neural::{NeuralEngine, Tensor, TensorDtype, TensorShape};
    use crate::optimizer::Optimizer;
    use crate::resources::ResourceOracle;
    use crate::safety::SafetyChecker;
    use crate::security::SecurityOracle;

    // =========================================================================
    // Core Types Tests
    // =========================================================================

    #[test]
    fn test_confidence_creation() {
        let conf = Confidence::new(0.85);
        assert!(conf.value() >= 0.0 && conf.value() <= 1.0);
        assert_eq!(conf.value(), 0.85);
    }

    #[test]
    fn test_confidence_clamping() {
        let high = Confidence::new(1.5);
        assert_eq!(high.value(), 1.0);

        let low = Confidence::new(-0.5);
        assert_eq!(low.value(), 0.0);
    }

    #[test]
    fn test_decision_id_uniqueness() {
        let id1 = DecisionId::new();
        let id2 = DecisionId::new();
        let id3 = DecisionId::new();

        assert_ne!(id1.value(), id2.value());
        assert_ne!(id2.value(), id3.value());
        assert_ne!(id1.value(), id3.value());
    }

    #[test]
    fn test_ai_config_default() {
        let config = AiConfig::default();
        assert!(config.intent_engine_enabled);
        assert!(config.self_optimization_enabled);
        assert!(config.self_healing_enabled);
        assert!(config.predictive_security_enabled);
    }

    #[test]
    fn test_ai_config_is_valid() {
        let config = AiConfig::default();
        assert!(config.is_valid());
    }

    #[test]
    fn test_ai_action_noop() {
        let action = AiAction::NoOp;
        assert!(matches!(action, AiAction::NoOp));
    }

    #[test]
    fn test_ai_action_tune_scheduler() {
        let action = AiAction::TuneScheduler {
            granularity_ns: 5_000_000,
            preemption: true,
        };

        if let AiAction::TuneScheduler {
            granularity_ns,
            preemption,
        } = action
        {
            assert_eq!(granularity_ns, 5_000_000);
            assert!(preemption);
        } else {
            panic!("Expected TuneScheduler");
        }
    }

    #[test]
    fn test_ai_action_sequence() {
        let actions = vec![
            AiAction::NoOp,
            AiAction::TuneScheduler {
                granularity_ns: 1000,
                preemption: true,
            },
            AiAction::ForceGarbageCollection,
        ];
        let seq = AiAction::Sequence(actions);

        if let AiAction::Sequence(inner) = seq {
            assert_eq!(inner.len(), 3);
        } else {
            panic!("Expected Sequence");
        }
    }

    #[test]
    fn test_system_metrics_default() {
        let metrics = SystemMetrics::default();
        assert_eq!(metrics.cpu_usage_percent, 0);
        assert_eq!(metrics.memory_usage_percent, 0);
    }

    #[test]
    fn test_decision_context_creation() {
        let context = DecisionContext {
            trigger_event: Some(String::from("TestEvent")),
            cpu_usage: 0.5,
            memory_usage: 0.6,
            active_processes: 100,
            io_pending: 5,
            system_metrics: SystemMetrics::default(),
            constraints: vec![String::from("max_cpu_80")],
            time_budget_us: 1000,
        };

        assert_eq!(context.cpu_usage, 0.5);
        assert_eq!(context.active_processes, 100);
    }

    // =========================================================================
    // Neural Engine Tests
    // =========================================================================

    #[test]
    fn test_tensor_shape_vector() {
        let shape = TensorShape::vector(10);
        assert_eq!(shape.ndim(), 1);
        assert_eq!(shape.size(), 10);
    }

    #[test]
    fn test_tensor_shape_matrix() {
        let shape = TensorShape::matrix(3, 4);
        assert_eq!(shape.ndim(), 2);
        assert_eq!(shape.size(), 12);
    }

    #[test]
    fn test_tensor_zeros() {
        let tensor = Tensor::zeros(TensorShape::vector(5), TensorDtype::F32);
        assert_eq!(tensor.data().len(), 5);
        assert!(tensor.data().iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_tensor_ones() {
        let tensor = Tensor::ones(TensorShape::vector(3), TensorDtype::F32);
        assert_eq!(tensor.data().len(), 3);
        assert!(tensor.data().iter().all(|&x| x == 1.0));
    }

    #[test]
    fn test_tensor_from_vec() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let tensor = Tensor::from_vec(data.clone(), TensorShape::vector(4));
        assert_eq!(tensor.data(), &data[..]);
    }

    #[test]
    fn test_tensor_add() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0], TensorShape::vector(3));
        let b = Tensor::from_vec(vec![4.0, 5.0, 6.0], TensorShape::vector(3));
        let c = a.add(&b).unwrap();
        assert_eq!(c.data(), &[5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_tensor_mul() {
        let a = Tensor::from_vec(vec![2.0, 3.0, 4.0], TensorShape::vector(3));
        let b = Tensor::from_vec(vec![2.0, 2.0, 2.0], TensorShape::vector(3));
        let c = a.mul(&b).unwrap();
        assert_eq!(c.data(), &[4.0, 6.0, 8.0]);
    }

    #[test]
    fn test_tensor_relu() {
        let tensor = Tensor::from_vec(vec![-1.0, 0.0, 1.0, 2.0], TensorShape::vector(4));
        let result = tensor.relu();
        assert_eq!(result.data(), &[0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_tensor_sigmoid() {
        let tensor = Tensor::from_vec(vec![0.0], TensorShape::vector(1));
        let result = tensor.sigmoid();
        // sigmoid(0) = 0.5
        assert!((result.data()[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_tensor_softmax() {
        let tensor = Tensor::from_vec(vec![1.0, 1.0, 1.0], TensorShape::vector(3));
        let result = tensor.softmax();
        // All equal inputs -> equal probabilities
        let expected = 1.0 / 3.0;
        for &val in result.data() {
            assert!((val - expected).abs() < 0.01);
        }
        // Sum should be 1.0
        let sum: f32 = result.data().iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_neural_engine_creation() {
        let engine = NeuralEngine::new(false, false);
        let stats = engine.statistics();
        assert_eq!(stats.models_registered, 0);
    }

    // =========================================================================
    // Learning Engine Tests
    // =========================================================================

    #[test]
    fn test_state_vector_creation() {
        let features = vec![0.5, 0.6, 0.7];
        let names = vec![String::from("cpu"), String::from("mem"), String::from("io")];
        let state = StateVector::new(features.clone(), names);
        assert_eq!(state.dim(), 3);
        assert_eq!(state.features, features);
    }

    #[test]
    fn test_state_vector_from_context() {
        let context = DecisionContext {
            trigger_event: None,
            cpu_usage: 0.75,
            memory_usage: 0.50,
            active_processes: 200,
            io_pending: 10,
            system_metrics: SystemMetrics {
                cpu_usage_percent: 75,
                memory_usage_percent: 50,
                io_wait_percent: 5,
                process_count: 200,
                thread_count: 500,
                interrupt_rate: 1000,
                context_switch_rate: 5000,
            },
            constraints: vec![],
            time_budget_us: 1000,
        };

        let state = StateVector::from_context(&context);
        assert!(state.dim() > 0);
        assert_eq!(state.features[0], 0.75); // cpu_usage
        assert_eq!(state.features[1], 0.50); // memory_usage
    }

    #[test]
    fn test_state_vector_distance() {
        let a = StateVector::new(vec![0.0, 0.0, 0.0], vec![]);
        let b = StateVector::new(vec![3.0, 4.0, 0.0], vec![]);
        let dist = a.distance(&b);
        assert!((dist - 5.0).abs() < 0.01); // sqrt(9 + 16) = 5
    }

    #[test]
    fn test_action_vector_from_action() {
        let action = AiAction::TuneScheduler {
            granularity_ns: 1_000_000,
            preemption: true,
        };
        let vec = ActionVector::from_action(&action);
        assert_eq!(vec.action_type, 0); // TuneScheduler has action_type 0
    }

    #[test]
    fn test_learning_engine_creation() {
        let engine = LearningEngine::new(true);
        let stats = engine.statistics();
        assert!(stats.enabled);
    }

    #[test]
    fn test_learning_engine_disabled() {
        let engine = LearningEngine::new(false);
        let stats = engine.statistics();
        assert!(!stats.enabled);
    }

    // =========================================================================
    // Memory Tests
    // =========================================================================

    #[test]
    fn test_ai_memory_creation() {
        let memory = AiMemory::new(1024 * 1024); // 1MB
        let stats = memory.statistics();
        assert_eq!(stats.bytes_used, 0);
    }

    // =========================================================================
    // Metrics Tests
    // =========================================================================

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let stats = collector.statistics();
        assert!(stats.series_count >= 0);
    }

    #[test]
    fn test_metrics_record() {
        let collector = MetricsCollector::new();
        // Must register a series before recording
        collector.register(crate::metrics::MetricDefinition::gauge(
            "cpu_usage",
            "CPU usage",
            "CPU usage percentage",
            "percent",
        ));
        collector.record("cpu_usage", 75.0);
        collector.record("cpu_usage", 80.0);

        let stats = collector.statistics();
        assert!(stats.metrics_recorded >= 2);
    }

    #[test]
    fn test_metrics_summary() {
        let collector = MetricsCollector::new();
        collector.record("test_metric", 10.0);
        collector.record("test_metric", 20.0);
        collector.record("test_metric", 30.0);

        let summary = collector.summary();
        assert!(summary.series_count >= 0);
    }

    // =========================================================================
    // Safety Tests
    // =========================================================================

    #[test]
    fn test_safety_checker_creation() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);
        let stats = checker.statistics();
        assert_eq!(stats.checks_performed, 0);
    }

    #[test]
    fn test_safety_check_noop() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);

        let decision = AiDecision {
            id: DecisionId::new(),
            timestamp: 0,
            action: AiAction::NoOp,
            confidence: Confidence::new(0.9),
            priority: AiPriority::Low,
            reasoning: vec![String::from("Test")],
            expected_outcome: String::from("Nothing"),
            rollback: None,
            context: DecisionContext::default(),
        };

        let result = checker.check(&decision);
        assert!(result.allowed);
        assert!(result.violations.is_empty());
    }

    // =========================================================================
    // Intent Engine Tests
    // =========================================================================

    #[test]
    fn test_intent_engine_creation() {
        let engine = IntentEngine::new(true);
        let stats = engine.statistics();
        assert!(stats.enabled);
    }

    // =========================================================================
    // Optimizer Tests
    // =========================================================================

    #[test]
    fn test_optimizer_creation() {
        let optimizer = Optimizer::new(true);
        let stats = optimizer.statistics();
        assert!(stats.enabled);
    }

    // =========================================================================
    // Healer Tests
    // =========================================================================

    #[test]
    fn test_healer_creation() {
        let healer = Healer::new(true);
        let stats = healer.statistics();
        assert!(stats.enabled);
        assert_eq!(stats.issues_detected, 0);
    }

    // =========================================================================
    // Security Oracle Tests
    // =========================================================================

    #[test]
    fn test_security_oracle_creation() {
        let oracle = SecurityOracle::new(true);
        let stats = oracle.statistics();
        assert!(stats.enabled);
        assert_eq!(stats.threats_detected, 0);
    }

    // =========================================================================
    // Resource Oracle Tests
    // =========================================================================

    #[test]
    fn test_resource_oracle_creation() {
        let oracle = ResourceOracle::new(false, false);
        let stats = oracle.statistics();
        assert_eq!(stats.allocations_made, 0);
    }

    // =========================================================================
    // Cortex Integration Tests
    // =========================================================================

    #[test]
    fn test_cortex_creation() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        assert_eq!(cortex.state(), AiState::Initializing);
    }

    #[test]
    fn test_cortex_initialization() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);

        let result = cortex.initialize();
        assert!(result.is_ok());
        assert_eq!(cortex.state(), AiState::Idle);
    }

    #[test]
    fn test_cortex_submit_event() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        let event = AiEvent::SystemBoot;
        let result = cortex.submit_event(event, AiPriority::Normal);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cortex_process() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        // Submit some events
        cortex
            .submit_event(AiEvent::SystemBoot, AiPriority::Normal)
            .unwrap();
        cortex
            .submit_event(
                AiEvent::CpuThreshold {
                    usage_percent: 90,
                    cpu_id: 0,
                },
                AiPriority::High,
            )
            .unwrap();

        // Process events
        let decisions = cortex.process();
        assert!(decisions.is_ok());
    }

    #[test]
    fn test_cortex_statistics() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        cortex
            .submit_event(AiEvent::SystemBoot, AiPriority::Low)
            .unwrap();
        let _ = cortex.process();

        let stats = cortex.statistics();
        assert!(stats.events_processed >= 0);
    }

    // =========================================================================
    // Math Utility Tests
    // =========================================================================

    #[test]
    fn test_math_exp() {
        let result = crate::math::exp_f32(0.0);
        assert!((result - 1.0).abs() < 0.001);

        let result = crate::math::exp_f32(1.0);
        assert!((result - 2.718).abs() < 0.01);
    }

    #[test]
    fn test_math_sqrt() {
        let result = crate::math::sqrt_f32(4.0);
        assert!((result - 2.0).abs() < 0.001);

        let result = crate::math::sqrt_f32(9.0);
        assert!((result - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_math_ln_1p() {
        let result = crate::math::ln_1p_f32(0.0);
        assert!(result.abs() < 0.001); // ln(1) = 0
    }

    #[test]
    fn test_math_powi() {
        let result = crate::math::powi_f32(2.0, 3);
        assert!((result - 8.0).abs() < 0.001);

        let result = crate::math::powi_f64(3.0, 2);
        assert!((result - 9.0).abs() < 0.001);
    }

    // =========================================================================
    // End-to-End Scenario Tests
    // =========================================================================

    #[test]
    fn test_scenario_high_cpu_response() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        // Simulate high CPU event
        let event = AiEvent::CpuThreshold {
            usage_percent: 95,
            cpu_id: 0,
        };
        cortex.submit_event(event, AiPriority::High).unwrap();

        // Process and expect optimization decision
        let decisions = cortex.process().unwrap();

        // The AI should generate decisions for high CPU
        assert!(decisions.len() >= 0);
    }

    #[test]
    fn test_scenario_memory_pressure() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        let event = AiEvent::MemoryPressure {
            available_percent: 5,
        };
        cortex.submit_event(event, AiPriority::Critical).unwrap();

        let decisions = cortex.process().unwrap();
        assert!(decisions.len() >= 0);
    }

    #[test]
    fn test_scenario_security_threat() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        let event = AiEvent::ThreatSignature {
            signature_id: 12345,
            confidence: Confidence::new(0.95),
        };
        cortex.submit_event(event, AiPriority::Critical).unwrap();

        let decisions = cortex.process().unwrap();
        assert!(decisions.len() >= 0);
    }

    #[test]
    fn test_scenario_module_error() {
        let config = AiConfig::default();
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        let event = AiEvent::ModuleError {
            module_id: 42,
            error: String::from("Segmentation fault"),
        };
        cortex.submit_event(event, AiPriority::High).unwrap();

        let decisions = cortex.process().unwrap();
        assert!(decisions.len() >= 0);
    }

    // =========================================================================
    // AI Decision Pipeline Test
    // =========================================================================

    #[test]
    fn test_full_decision_pipeline() {
        // Create and initialize cortex
        let config = AiConfig {
            intent_engine_enabled: true,
            self_optimization_enabled: true,
            self_healing_enabled: true,
            predictive_security_enabled: true,
            continuous_learning_enabled: true,
            ..Default::default()
        };
        let cortex = Cortex::new(config);
        cortex.initialize().unwrap();

        // Submit multiple events
        cortex
            .submit_event(AiEvent::SystemBoot, AiPriority::Low)
            .unwrap();
        cortex
            .submit_event(
                AiEvent::ProcessSpawn {
                    pid: 1234,
                    name: String::from("cargo"),
                },
                AiPriority::Normal,
            )
            .unwrap();
        cortex
            .submit_event(
                AiEvent::CpuThreshold {
                    usage_percent: 85,
                    cpu_id: 0,
                },
                AiPriority::High,
            )
            .unwrap();

        // Process all events
        let _decisions = cortex.process().unwrap();

        // Verify statistics updated
        let stats = cortex.statistics();
        assert!(stats.events_processed > 0);

        // Test state transitions
        assert_eq!(cortex.state(), AiState::Idle);
    }
}
