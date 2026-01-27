//! Benchmark Results & Reporting
//!
//! Collects, analyzes, and formats benchmark results.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use core::fmt::Write;

use crate::{BenchmarkCategory, BenchmarkId, Statistics, engine::RunResult};

// =============================================================================
// Result Types
// =============================================================================

/// Complete benchmark report
#[derive(Clone)]
pub struct BenchmarkReport {
    /// Title of the report
    pub title: String,
    /// Platform information
    pub platform: PlatformInfo,
    /// Configuration used
    pub config: ReportConfig,
    /// Results by category
    pub categories: Vec<CategoryResults>,
    /// Summary statistics
    pub summary: ReportSummary,
}

impl BenchmarkReport {
    /// Create a report from BenchmarkResults
    pub fn from_results(results: Vec<crate::BenchmarkResults>, config: &crate::BenchmarkConfig) -> Self {
        let mut categories_map: Vec<(BenchmarkCategory, Vec<BenchmarkResult>)> = Vec::new();
        
        for result in &results {
            let category = result.category;
            
            // Convert BenchmarkResults to BenchmarkResult
            let bench_result = BenchmarkResult {
                id: result.id.clone(),
                name: result.name.clone(),
                stats: result.stats.clone(),
                status: if result.failed { BenchmarkStatus::Failed } else { BenchmarkStatus::Passed },
                cycles: CycleStats {
                    min: result.stats.min,
                    max: result.stats.max,
                    mean: result.stats.mean,
                    median: result.stats.p50,
                    p95: result.stats.p95,
                    p99: result.stats.p99,
                    std_dev: result.stats.std_dev,
                },
                time: TimeStats {
                    min_ns: result.stats.min * 1000 / config.cpu_freq_mhz,
                    max_ns: result.stats.max * 1000 / config.cpu_freq_mhz,
                    mean_ns: result.stats.mean * 1000 / config.cpu_freq_mhz,
                    median_ns: result.stats.p50 * 1000 / config.cpu_freq_mhz,
                    p95_ns: result.stats.p95 * 1000 / config.cpu_freq_mhz,
                    p99_ns: result.stats.p99 * 1000 / config.cpu_freq_mhz,
                    std_dev_ns: result.stats.std_dev * 1000 / config.cpu_freq_mhz,
                },
            };
            
            // Find or create category
            let found = categories_map.iter_mut()
                .find(|(cat, _)| *cat == category);
            
            if let Some((_, benchmarks)) = found {
                benchmarks.push(bench_result);
            } else {
                categories_map.push((category, vec![bench_result]));
            }
        }
        
        // Build category results with totals
        let mut categories = Vec::new();
        for (category, benchmarks) in categories_map {
            let totals = ResultCollector::compute_totals(&benchmarks);
            categories.push(CategoryResults {
                category,
                benchmarks,
                totals,
            });
        }
        
        // Build summary
        let mut summary = ReportSummary::default();
        let mut fastest_time = u64::MAX;
        let mut slowest_time = 0u64;
        
        for cat in &categories {
            summary.total_benchmarks += cat.totals.total_benchmarks;
            summary.passed += cat.totals.passed;
            summary.warnings += cat.totals.warnings;
            summary.failed += cat.totals.failed;
            summary.skipped += cat.totals.skipped;
            summary.total_time_ms += cat.totals.total_time_us / 1000;
            
            for bench in &cat.benchmarks {
                if bench.time.mean_ns < fastest_time && bench.time.mean_ns > 0 {
                    fastest_time = bench.time.mean_ns;
                    summary.fastest_benchmark = bench.name.clone();
                }
                if bench.time.mean_ns > slowest_time {
                    slowest_time = bench.time.mean_ns;
                    summary.slowest_benchmark = bench.name.clone();
                }
            }
        }
        
        summary.performance_score = if summary.total_benchmarks > 0 {
            (summary.passed * 100) / summary.total_benchmarks
        } else {
            0
        };
        
        Self {
            title: String::from("Helix Kernel Benchmark Report"),
            platform: PlatformInfo::default(),
            config: ReportConfig {
                iterations: config.iterations,
                warmup_iterations: config.warmup_iterations,
                cpu_freq_hz: config.cpu_freq_mhz * 1_000_000,
                timestamp: 0,
            },
            categories,
            summary,
        }
    }
}

/// Platform information
#[derive(Clone)]
pub struct PlatformInfo {
    pub arch: String,
    pub cpu_model: String,
    pub cpu_freq_mhz: u64,
    pub cores: u32,
    pub memory_mb: u64,
    pub virtualized: bool,
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self {
            arch: String::from("x86_64"),
            cpu_model: String::from("Unknown"),
            cpu_freq_mhz: 2500,
            cores: 1,
            memory_mb: 256,
            virtualized: true,
        }
    }
}

/// Report configuration
#[derive(Clone)]
pub struct ReportConfig {
    pub iterations: u32,
    pub warmup_iterations: u32,
    pub cpu_freq_hz: u64,
    pub timestamp: u64,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            iterations: 10_000,
            warmup_iterations: 1_000,
            cpu_freq_hz: 2_500_000_000,
            timestamp: 0,
        }
    }
}

/// Results for a single category
#[derive(Clone)]
pub struct CategoryResults {
    pub category: BenchmarkCategory,
    pub benchmarks: Vec<BenchmarkResult>,
    pub totals: CategoryTotals,
}

/// Single benchmark result
#[derive(Clone)]
pub struct BenchmarkResult {
    pub id: BenchmarkId,
    pub name: String,
    pub stats: Statistics,
    pub status: BenchmarkStatus,
    pub cycles: CycleStats,
    pub time: TimeStats,
}

/// Benchmark execution status
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

/// Cycle statistics
#[derive(Clone, Default)]
pub struct CycleStats {
    pub min: u64,
    pub max: u64,
    pub mean: u64,
    pub median: u64,
    pub p95: u64,
    pub p99: u64,
    pub std_dev: u64,
}

/// Time statistics (nanoseconds)
#[derive(Clone, Default)]
pub struct TimeStats {
    pub min_ns: u64,
    pub max_ns: u64,
    pub mean_ns: u64,
    pub median_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub std_dev_ns: u64,
}

/// Category totals
#[derive(Clone, Default)]
pub struct CategoryTotals {
    pub total_benchmarks: u32,
    pub passed: u32,
    pub warnings: u32,
    pub failed: u32,
    pub skipped: u32,
    pub total_cycles: u64,
    pub total_time_us: u64,
}

/// Report summary
#[derive(Clone, Default)]
pub struct ReportSummary {
    pub total_benchmarks: u32,
    pub passed: u32,
    pub warnings: u32,
    pub failed: u32,
    pub skipped: u32,
    pub fastest_benchmark: String,
    pub slowest_benchmark: String,
    pub total_time_ms: u64,
    pub performance_score: u32,
}

// =============================================================================
// Result Collection
// =============================================================================

/// Result collector
pub struct ResultCollector {
    results: Vec<(BenchmarkId, RunResult)>,
    cpu_freq_hz: u64,
}

impl ResultCollector {
    /// Create new collector
    pub fn new(cpu_freq_hz: u64) -> Self {
        Self {
            results: Vec::new(),
            cpu_freq_hz,
        }
    }
    
    /// Add a result
    pub fn add(&mut self, id: BenchmarkId, result: RunResult) {
        self.results.push((id, result));
    }
    
    /// Get number of results
    pub fn len(&self) -> usize {
        self.results.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
    
    /// Build report
    pub fn build_report(self, title: &str, platform: PlatformInfo, config: ReportConfig) -> BenchmarkReport {
        let mut categories_map: Vec<(BenchmarkCategory, Vec<BenchmarkResult>)> = Vec::new();
        
        for (id, run_result) in &self.results {
            let category = id.category;
            
            // Convert to benchmark result
            let bench_result = self.convert_result(id.clone(), run_result);
            
            // Find or create category
            let found = categories_map.iter_mut()
                .find(|(cat, _)| *cat == category);
            
            if let Some((_, results)) = found {
                results.push(bench_result);
            } else {
                categories_map.push((category, vec![bench_result]));
            }
        }
        
        // Build category results
        let mut categories = Vec::new();
        for (category, benchmarks) in categories_map {
            let totals = Self::compute_totals(&benchmarks);
            categories.push(CategoryResults {
                category,
                benchmarks,
                totals,
            });
        }
        
        // Build summary
        let summary = self.build_summary(&categories);
        
        BenchmarkReport {
            title: String::from(title),
            platform,
            config,
            categories,
            summary,
        }
    }
    
    /// Convert run result to benchmark result
    fn convert_result(&self, id: BenchmarkId, run: &RunResult) -> BenchmarkResult {
        let cycles = CycleStats {
            min: run.min_cycles,
            max: run.max_cycles,
            mean: run.mean_cycles,
            median: run.median_cycles,
            p95: run.p95_cycles,
            p99: run.p99_cycles,
            std_dev: run.std_dev_cycles,
        };
        
        let time = TimeStats {
            min_ns: self.cycles_to_ns(run.min_cycles),
            max_ns: self.cycles_to_ns(run.max_cycles),
            mean_ns: self.cycles_to_ns(run.mean_cycles),
            median_ns: self.cycles_to_ns(run.median_cycles),
            p95_ns: self.cycles_to_ns(run.p95_cycles),
            p99_ns: self.cycles_to_ns(run.p99_cycles),
            std_dev_ns: self.cycles_to_ns(run.std_dev_cycles),
        };
        
        // Determine status based on jitter
        let jitter_ratio = if run.mean_cycles > 0 {
            (run.std_dev_cycles * 100) / run.mean_cycles
        } else {
            0
        };
        
        let status = if jitter_ratio > 50 {
            BenchmarkStatus::Warning
        } else {
            BenchmarkStatus::Passed
        };
        
        // Build stats for the result
        let stats = crate::Statistics {
            min: run.min_cycles,
            max: run.max_cycles,
            mean: run.mean_cycles,
            p50: run.median_cycles,
            p95: run.p95_cycles,
            p99: run.p99_cycles,
            std_dev: run.std_dev_cycles,
            variance: run.std_dev_cycles * run.std_dev_cycles,
            jitter: run.max_cycles.saturating_sub(run.min_cycles),
        };
        
        BenchmarkResult {
            id: id.clone(),
            name: id.name,
            stats,
            status,
            cycles,
            time,
        }
    }
    
    /// Compute category totals
    fn compute_totals(benchmarks: &[BenchmarkResult]) -> CategoryTotals {
        let mut totals = CategoryTotals::default();
        
        for bench in benchmarks {
            totals.total_benchmarks += 1;
            totals.total_cycles += bench.cycles.mean;
            totals.total_time_us += bench.time.mean_ns / 1000;
            
            match bench.status {
                BenchmarkStatus::Passed => totals.passed += 1,
                BenchmarkStatus::Warning => totals.warnings += 1,
                BenchmarkStatus::Failed => totals.failed += 1,
                BenchmarkStatus::Skipped => totals.skipped += 1,
            }
        }
        
        totals
    }
    
    /// Build report summary
    fn build_summary(&self, categories: &[CategoryResults]) -> ReportSummary {
        let mut summary = ReportSummary::default();
        
        let mut fastest_time = u64::MAX;
        let mut slowest_time = 0u64;
        
        for cat in categories {
            summary.total_benchmarks += cat.totals.total_benchmarks;
            summary.passed += cat.totals.passed;
            summary.warnings += cat.totals.warnings;
            summary.failed += cat.totals.failed;
            summary.skipped += cat.totals.skipped;
            summary.total_time_ms += cat.totals.total_time_us / 1000;
            
            for bench in &cat.benchmarks {
                if bench.time.mean_ns < fastest_time {
                    fastest_time = bench.time.mean_ns;
                    summary.fastest_benchmark = bench.name.clone();
                }
                if bench.time.mean_ns > slowest_time {
                    slowest_time = bench.time.mean_ns;
                    summary.slowest_benchmark = bench.name.clone();
                }
            }
        }
        
        // Compute performance score (0-100)
        let pass_rate = if summary.total_benchmarks > 0 {
            (summary.passed * 100) / summary.total_benchmarks
        } else {
            0
        };
        summary.performance_score = pass_rate;
        
        summary
    }
    
    /// Convert cycles to nanoseconds
    fn cycles_to_ns(&self, cycles: u64) -> u64 {
        if self.cpu_freq_hz == 0 {
            return cycles;
        }
        (cycles * 1_000_000_000) / self.cpu_freq_hz
    }
}

// =============================================================================
// Report Formatting
// =============================================================================

/// Report formatter
pub struct ReportFormatter;

impl ReportFormatter {
    /// Format report as text
    pub fn format_text(report: &BenchmarkReport) -> String {
        let mut output = String::new();
        
        // Header
        writeln!(output, "╔══════════════════════════════════════════════════════════════════════╗").unwrap();
        writeln!(output, "║                    HELIX KERNEL BENCHMARK REPORT                      ║").unwrap();
        writeln!(output, "╠══════════════════════════════════════════════════════════════════════╣").unwrap();
        
        // Platform info
        writeln!(output, "║ Platform: {} ({} MHz, {} cores)                    ║", 
            report.platform.arch, report.platform.cpu_freq_mhz, report.platform.cores).unwrap();
        writeln!(output, "║ Config: {} iterations, {} warmup                              ║",
            report.config.iterations, report.config.warmup_iterations).unwrap();
        writeln!(output, "╠══════════════════════════════════════════════════════════════════════╣").unwrap();
        
        // Categories
        for cat in &report.categories {
            Self::format_category(&mut output, cat);
        }
        
        // Summary
        writeln!(output, "╠══════════════════════════════════════════════════════════════════════╣").unwrap();
        writeln!(output, "║                              SUMMARY                                  ║").unwrap();
        writeln!(output, "╠══════════════════════════════════════════════════════════════════════╣").unwrap();
        writeln!(output, "║ Total: {} | Passed: {} | Warnings: {} | Failed: {}               ║",
            report.summary.total_benchmarks, report.summary.passed,
            report.summary.warnings, report.summary.failed).unwrap();
        writeln!(output, "║ Fastest: {}                                          ║",
            report.summary.fastest_benchmark).unwrap();
        writeln!(output, "║ Slowest: {}                                          ║",
            report.summary.slowest_benchmark).unwrap();
        writeln!(output, "║ Performance Score: {}/100                                           ║",
            report.summary.performance_score).unwrap();
        writeln!(output, "╚══════════════════════════════════════════════════════════════════════╝").unwrap();
        
        output
    }
    
    /// Format a category
    fn format_category(output: &mut String, cat: &CategoryResults) {
        writeln!(output, "║                                                                        ║").unwrap();
        writeln!(output, "║ {:?} Benchmarks ({} tests)                                  ║",
            cat.category, cat.totals.total_benchmarks).unwrap();
        writeln!(output, "║────────────────────────────────────────────────────────────────────────║").unwrap();
        writeln!(output, "║ Name                          │ Mean (ns) │ P95 (ns)  │ Jitter (%)   ║").unwrap();
        writeln!(output, "║───────────────────────────────┼───────────┼───────────┼──────────────║").unwrap();
        
        for bench in &cat.benchmarks {
            let jitter = if bench.cycles.mean > 0 {
                (bench.cycles.std_dev * 100) / bench.cycles.mean
            } else {
                0
            };
            
            let status_icon = match bench.status {
                BenchmarkStatus::Passed => "✓",
                BenchmarkStatus::Warning => "⚠",
                BenchmarkStatus::Failed => "✗",
                BenchmarkStatus::Skipped => "○",
            };
            
            // Truncate name to 26 chars
            let name = if bench.name.len() > 26 {
                format!("{}...", &bench.name[..23])
            } else {
                bench.name.clone()
            };
            
            writeln!(output, "║ {} {:26} │ {:9} │ {:9} │ {:10}%  ║",
                status_icon, name, bench.time.mean_ns, bench.time.p95_ns, jitter).unwrap();
        }
    }
    
    /// Format report as markdown
    pub fn format_markdown(report: &BenchmarkReport) -> String {
        let mut output = String::new();
        
        writeln!(output, "# {}", report.title).unwrap();
        writeln!(output).unwrap();
        
        // Platform
        writeln!(output, "## Platform").unwrap();
        writeln!(output, "| Property | Value |").unwrap();
        writeln!(output, "|----------|-------|").unwrap();
        writeln!(output, "| Architecture | {} |", report.platform.arch).unwrap();
        writeln!(output, "| CPU | {} @ {} MHz |", report.platform.cpu_model, report.platform.cpu_freq_mhz).unwrap();
        writeln!(output, "| Cores | {} |", report.platform.cores).unwrap();
        writeln!(output, "| Memory | {} MB |", report.platform.memory_mb).unwrap();
        writeln!(output, "| Virtualized | {} |", report.platform.virtualized).unwrap();
        writeln!(output).unwrap();
        
        // Configuration
        writeln!(output, "## Configuration").unwrap();
        writeln!(output, "- Iterations: {}", report.config.iterations).unwrap();
        writeln!(output, "- Warmup: {}", report.config.warmup_iterations).unwrap();
        writeln!(output).unwrap();
        
        // Results by category
        for cat in &report.categories {
            writeln!(output, "## {:?}", cat.category).unwrap();
            writeln!(output).unwrap();
            writeln!(output, "| Benchmark | Mean (ns) | P95 (ns) | P99 (ns) | Jitter (%) |").unwrap();
            writeln!(output, "|-----------|-----------|----------|----------|------------|").unwrap();
            
            for bench in &cat.benchmarks {
                let jitter = if bench.cycles.mean > 0 {
                    (bench.cycles.std_dev * 100) / bench.cycles.mean
                } else {
                    0
                };
                
                let status = match bench.status {
                    BenchmarkStatus::Passed => "✅",
                    BenchmarkStatus::Warning => "⚠️",
                    BenchmarkStatus::Failed => "❌",
                    BenchmarkStatus::Skipped => "⏭️",
                };
                
                writeln!(output, "| {} {} | {} | {} | {} | {}% |",
                    status, bench.name,
                    bench.time.mean_ns, bench.time.p95_ns, bench.time.p99_ns,
                    jitter).unwrap();
            }
            writeln!(output).unwrap();
        }
        
        // Summary
        writeln!(output, "## Summary").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "- **Total Benchmarks**: {}", report.summary.total_benchmarks).unwrap();
        writeln!(output, "- **Passed**: {} ✅", report.summary.passed).unwrap();
        writeln!(output, "- **Warnings**: {} ⚠️", report.summary.warnings).unwrap();
        writeln!(output, "- **Failed**: {} ❌", report.summary.failed).unwrap();
        writeln!(output, "- **Fastest**: `{}`", report.summary.fastest_benchmark).unwrap();
        writeln!(output, "- **Slowest**: `{}`", report.summary.slowest_benchmark).unwrap();
        writeln!(output, "- **Performance Score**: {}/100", report.summary.performance_score).unwrap();
        
        output
    }
    
    /// Format compact table
    pub fn format_compact(report: &BenchmarkReport) -> String {
        let mut output = String::new();
        
        writeln!(output, "Helix Benchmark Results").unwrap();
        writeln!(output, "========================").unwrap();
        writeln!(output).unwrap();
        
        for cat in &report.categories {
            writeln!(output, "[{:?}]", cat.category).unwrap();
            
            for bench in &cat.benchmarks {
                writeln!(output, "  {}: {}ns (p99: {}ns)",
                    bench.name, bench.time.mean_ns, bench.time.p99_ns).unwrap();
            }
            writeln!(output).unwrap();
        }
        
        writeln!(output, "Score: {}/100", report.summary.performance_score).unwrap();
        
        output
    }
}

// =============================================================================
// Comparison
// =============================================================================

/// Compare two benchmark reports
pub struct ReportComparison {
    pub baseline: String,
    pub current: String,
    pub improvements: Vec<Improvement>,
    pub regressions: Vec<Regression>,
    pub unchanged: u32,
}

/// Performance improvement
pub struct Improvement {
    pub benchmark: String,
    pub baseline_ns: u64,
    pub current_ns: u64,
    pub improvement_pct: i32,
}

/// Performance regression
pub struct Regression {
    pub benchmark: String,
    pub baseline_ns: u64,
    pub current_ns: u64,
    pub regression_pct: i32,
}

impl ReportComparison {
    /// Compare two reports
    pub fn compare(baseline: &BenchmarkReport, current: &BenchmarkReport) -> Self {
        let mut improvements = Vec::new();
        let mut regressions = Vec::new();
        let mut unchanged = 0u32;
        
        // Build map of baseline results
        let mut baseline_map: Vec<(&str, u64)> = Vec::new();
        for cat in &baseline.categories {
            for bench in &cat.benchmarks {
                baseline_map.push((&bench.name, bench.time.mean_ns));
            }
        }
        
        // Compare current results
        for cat in &current.categories {
            for bench in &cat.benchmarks {
                let baseline_result = baseline_map.iter()
                    .find(|(name, _)| *name == bench.name.as_str());
                
                if let Some((_, baseline_ns)) = baseline_result {
                    let current_ns = bench.time.mean_ns;
                    
                    if *baseline_ns == 0 {
                        unchanged += 1;
                        continue;
                    }
                    
                    let diff_pct = ((current_ns as i64 - *baseline_ns as i64) * 100 / *baseline_ns as i64) as i32;
                    
                    if diff_pct < -5 {
                        // Improvement (faster)
                        improvements.push(Improvement {
                            benchmark: bench.name.clone(),
                            baseline_ns: *baseline_ns,
                            current_ns,
                            improvement_pct: -diff_pct,
                        });
                    } else if diff_pct > 5 {
                        // Regression (slower)
                        regressions.push(Regression {
                            benchmark: bench.name.clone(),
                            baseline_ns: *baseline_ns,
                            current_ns,
                            regression_pct: diff_pct,
                        });
                    } else {
                        unchanged += 1;
                    }
                }
            }
        }
        
        Self {
            baseline: baseline.title.clone(),
            current: current.title.clone(),
            improvements,
            regressions,
            unchanged,
        }
    }
    
    /// Format comparison
    pub fn format(&self) -> String {
        let mut output = String::new();
        
        writeln!(output, "Performance Comparison").unwrap();
        writeln!(output, "Baseline: {} vs Current: {}", self.baseline, self.current).unwrap();
        writeln!(output).unwrap();
        
        if !self.improvements.is_empty() {
            writeln!(output, "Improvements:").unwrap();
            for imp in &self.improvements {
                writeln!(output, "  ✓ {}: {}ns → {}ns ({:+}%)",
                    imp.benchmark, imp.baseline_ns, imp.current_ns, imp.improvement_pct).unwrap();
            }
            writeln!(output).unwrap();
        }
        
        if !self.regressions.is_empty() {
            writeln!(output, "Regressions:").unwrap();
            for reg in &self.regressions {
                writeln!(output, "  ✗ {}: {}ns → {}ns (+{}%)",
                    reg.benchmark, reg.baseline_ns, reg.current_ns, reg.regression_pct).unwrap();
            }
            writeln!(output).unwrap();
        }
        
        writeln!(output, "Unchanged: {}", self.unchanged).unwrap();
        
        output
    }
}
