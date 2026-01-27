#!/bin/bash
# =============================================================================
# Helix OS Framework - Module Manager
# =============================================================================
# Add, remove, and manage kernel modules
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/colors.sh"
source "${SCRIPT_DIR}/lib/logging.sh"
source "${SCRIPT_DIR}/lib/utils.sh"

HELIX_ROOT="${HELIX_ROOT:-$(get_project_root)}"
MODULES_DIR="${HELIX_ROOT}/modules_impl"

# =============================================================================
# Module Templates
# =============================================================================

create_scheduler_module() {
    local name="$1"
    local dir="${MODULES_DIR}/schedulers/${name}"
    
    mkdir -p "${dir}/src"
    
    # Cargo.toml
    cat > "${dir}/Cargo.toml" << EOF
[package]
name = "helix-scheduler-${name}"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
description = "${name} scheduler module for Helix OS Framework"

[dependencies]
helix-modules = { workspace = true }
helix-execution = { workspace = true }
helix-hal = { workspace = true }

log = { workspace = true }
spin = { workspace = true }

[features]
default = []
EOF

    # lib.rs
    cat > "${dir}/src/lib.rs" << 'EOF'
//! ${NAME} Scheduler Module
//!
//! A custom scheduler implementation for Helix OS.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod scheduler;

pub use scheduler::*;

use helix_modules::{
    Module, ModuleContext, ModuleId, ModuleVersion,
    ModuleFlags, ModuleMetadata, ModuleDependency, define_module,
};
use alloc::sync::Arc;

pub static METADATA: ModuleMetadata = ModuleMetadata {
    id: ModuleId::from_raw(0x${RANDOM_ID}),
    name: "${NAME} Scheduler",
    version: ModuleVersion::new(1, 0, 0),
    description: "Custom ${NAME} scheduler implementation",
    author: "Helix OS Team",
    license: "MIT OR Apache-2.0",
    flags: ModuleFlags::KERNEL_SPACE,
};

pub struct ${NAME_PASCAL}Module {
    scheduler: Option<Arc<${NAME_PASCAL}Scheduler>>,
}

impl ${NAME_PASCAL}Module {
    pub fn new() -> Self {
        Self { scheduler: None }
    }
}

impl Default for ${NAME_PASCAL}Module {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ${NAME_PASCAL}Module {
    fn metadata(&self) -> &ModuleMetadata {
        &METADATA
    }

    fn init(&mut self, ctx: &ModuleContext) -> Result<(), &'static str> {
        log::info!("Initializing ${NAME} scheduler");
        let cpu_count = ctx.get_config::<usize>("cpu_count").unwrap_or(1);
        let mut scheduler = ${NAME_PASCAL}Scheduler::new();
        scheduler.init(cpu_count).map_err(|_| "Failed to init scheduler")?;
        self.scheduler = Some(Arc::new(scheduler));
        Ok(())
    }

    fn start(&mut self) -> Result<(), &'static str> {
        if let Some(ref scheduler) = self.scheduler {
            helix_execution::scheduler::framework().set_scheduler(scheduler.clone());
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), &'static str> {
        self.scheduler = None;
        Ok(())
    }

    fn dependencies(&self) -> &[ModuleDependency] {
        &[]
    }

    fn provides(&self) -> &[&str] {
        &["scheduler"]
    }

    fn save_state(&self) -> Option<alloc::vec::Vec<u8>> {
        None
    }

    fn restore_state(&mut self, _state: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

define_module!(${NAME_PASCAL}Module);
EOF

    # scheduler.rs
    cat > "${dir}/src/scheduler.rs" << 'EOF'
//! ${NAME} Scheduler Implementation

use helix_execution::{ThreadId, ExecResult, ExecError};
use helix_execution::scheduler::{
    Scheduler, SchedulableThread, SchedulerStats, Priority, SchedulingPolicy,
};
use alloc::collections::BTreeMap;
use spin::RwLock;

pub struct ${NAME_PASCAL}Scheduler {
    threads: RwLock<BTreeMap<ThreadId, SchedulableThread>>,
    cpu_count: usize,
}

impl ${NAME_PASCAL}Scheduler {
    pub fn new() -> Self {
        Self {
            threads: RwLock::new(BTreeMap::new()),
            cpu_count: 1,
        }
    }
}

impl Default for ${NAME_PASCAL}Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for ${NAME_PASCAL}Scheduler {
    fn name(&self) -> &'static str {
        "${NAME}"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn init(&mut self, cpu_count: usize) -> ExecResult<()> {
        self.cpu_count = cpu_count;
        log::info!("${NAME} scheduler initialized for {} CPUs", cpu_count);
        Ok(())
    }

    fn pick_next(&self, _cpu: usize) -> Option<ThreadId> {
        // TODO: Implement your scheduling algorithm here
        self.threads.read().keys().next().copied()
    }

    fn add_thread(&self, thread: SchedulableThread) -> ExecResult<()> {
        self.threads.write().insert(thread.id, thread);
        Ok(())
    }

    fn remove_thread(&self, id: ThreadId) -> ExecResult<()> {
        self.threads.write().remove(&id);
        Ok(())
    }

    fn thread_ready(&self, _id: ThreadId) -> ExecResult<()> {
        Ok(())
    }

    fn thread_block(&self, _id: ThreadId) -> ExecResult<()> {
        Ok(())
    }

    fn yield_thread(&self, _cpu: usize) {}

    fn tick(&self, _cpu: usize) {}

    fn set_priority(&self, _id: ThreadId, _priority: Priority) -> ExecResult<()> {
        Ok(())
    }

    fn get_priority(&self, id: ThreadId) -> Option<Priority> {
        self.threads.read().get(&id).map(|t| t.priority)
    }

    fn needs_reschedule(&self, _cpu: usize) -> bool {
        false
    }

    fn stats(&self) -> SchedulerStats {
        SchedulerStats::default()
    }
}
EOF

    # Replace placeholders
    local name_pascal=$(echo "${name}" | sed -r 's/(^|_)([a-z])/\U\2/g')
    local random_id=$(printf '%08X' $RANDOM$RANDOM)
    
    sed -i "s/\${NAME}/${name}/g" "${dir}/src/lib.rs" "${dir}/src/scheduler.rs"
    sed -i "s/\${NAME_PASCAL}/${name_pascal}/g" "${dir}/src/lib.rs" "${dir}/src/scheduler.rs"
    sed -i "s/\${RANDOM_ID}/${random_id}/g" "${dir}/src/lib.rs"
}

create_allocator_module() {
    local name="$1"
    local dir="${MODULES_DIR}/allocators/${name}"
    
    mkdir -p "${dir}/src"
    
    # Cargo.toml
    cat > "${dir}/Cargo.toml" << EOF
[package]
name = "helix-allocator-${name}"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
description = "${name} allocator module for Helix OS Framework"

[dependencies]
helix-modules = { workspace = true }
helix-memory = { workspace = true }
helix-hal = { workspace = true }

log = { workspace = true }
spin = { workspace = true }

[features]
default = []
EOF

    # lib.rs
    cat > "${dir}/src/lib.rs" << 'EOF'
//! ${NAME} Allocator Module

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod allocator;

pub use allocator::*;

use helix_modules::{
    Module, ModuleContext, ModuleId, ModuleVersion,
    ModuleFlags, ModuleMetadata, ModuleDependency, define_module,
};

pub static METADATA: ModuleMetadata = ModuleMetadata {
    id: ModuleId::from_raw(0x${RANDOM_ID}),
    name: "${NAME} Allocator",
    version: ModuleVersion::new(1, 0, 0),
    description: "Custom ${NAME} allocator implementation",
    author: "Helix OS Team",
    license: "MIT OR Apache-2.0",
    flags: ModuleFlags::KERNEL_SPACE,
};

pub struct ${NAME_PASCAL}Module;

impl Default for ${NAME_PASCAL}Module {
    fn default() -> Self { Self }
}

impl Module for ${NAME_PASCAL}Module {
    fn metadata(&self) -> &ModuleMetadata { &METADATA }
    fn init(&mut self, _ctx: &ModuleContext) -> Result<(), &'static str> { Ok(()) }
    fn start(&mut self) -> Result<(), &'static str> { Ok(()) }
    fn stop(&mut self) -> Result<(), &'static str> { Ok(()) }
    fn cleanup(&mut self) -> Result<(), &'static str> { Ok(()) }
    fn dependencies(&self) -> &[ModuleDependency] { &[] }
    fn provides(&self) -> &[&str] { &["allocator"] }
    fn save_state(&self) -> Option<alloc::vec::Vec<u8>> { None }
    fn restore_state(&mut self, _state: &[u8]) -> Result<(), &'static str> { Ok(()) }
}

define_module!(${NAME_PASCAL}Module);
EOF

    # allocator.rs
    cat > "${dir}/src/allocator.rs" << 'EOF'
//! ${NAME} Allocator Implementation

use helix_memory::{Frame, MemResult, MemError, MemoryZone};
use helix_memory::physical::{PhysicalAllocator, PhysicalRegion, AllocatorStats};
use helix_hal::PageSize;
use spin::Mutex;

pub struct ${NAME_PASCAL}Allocator {
    // Add your allocator state here
}

impl ${NAME_PASCAL}Allocator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ${NAME_PASCAL}Allocator {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicalAllocator for ${NAME_PASCAL}Allocator {
    fn name(&self) -> &'static str {
        "${NAME}"
    }

    fn init(&mut self, _regions: &[PhysicalRegion]) -> MemResult<()> {
        // TODO: Initialize allocator with memory regions
        Ok(())
    }

    fn allocate(&self, _size: PageSize) -> MemResult<Frame> {
        // TODO: Implement allocation
        Err(MemError::OutOfMemory)
    }

    fn allocate_contiguous(&self, _count: usize, size: PageSize) -> MemResult<Frame> {
        self.allocate(size)
    }

    fn allocate_zone(&self, size: PageSize, _zone: MemoryZone) -> MemResult<Frame> {
        self.allocate(size)
    }

    fn deallocate(&self, _frame: Frame) -> MemResult<()> {
        // TODO: Implement deallocation
        Ok(())
    }

    fn free_frames(&self) -> usize { 0 }
    fn total_frames(&self) -> usize { 0 }
    fn stats(&self) -> AllocatorStats { AllocatorStats::default() }
}
EOF

    # Replace placeholders
    local name_pascal=$(echo "${name}" | sed -r 's/(^|_)([a-z])/\U\2/g')
    local random_id=$(printf '%08X' $RANDOM$RANDOM)
    
    sed -i "s/\${NAME}/${name}/g" "${dir}/src/lib.rs" "${dir}/src/allocator.rs"
    sed -i "s/\${NAME_PASCAL}/${name_pascal}/g" "${dir}/src/lib.rs" "${dir}/src/allocator.rs"
    sed -i "s/\${RANDOM_ID}/${random_id}/g" "${dir}/src/lib.rs"
}

create_driver_module() {
    local name="$1"
    local dir="${MODULES_DIR}/drivers/${name}"
    
    mkdir -p "${dir}/src"
    
    # Cargo.toml
    cat > "${dir}/Cargo.toml" << EOF
[package]
name = "helix-driver-${name}"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
description = "${name} driver module for Helix OS Framework"

[dependencies]
helix-modules = { workspace = true }
helix-hal = { workspace = true }

log = { workspace = true }
spin = { workspace = true }

[features]
default = []
EOF

    # lib.rs
    cat > "${dir}/src/lib.rs" << 'EOF'
//! ${NAME} Driver Module

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

use helix_modules::{
    Module, ModuleContext, ModuleId, ModuleVersion,
    ModuleFlags, ModuleMetadata, ModuleDependency, define_module,
};

pub static METADATA: ModuleMetadata = ModuleMetadata {
    id: ModuleId::from_raw(0x${RANDOM_ID}),
    name: "${NAME} Driver",
    version: ModuleVersion::new(1, 0, 0),
    description: "${NAME} device driver",
    author: "Helix OS Team",
    license: "MIT OR Apache-2.0",
    flags: ModuleFlags::KERNEL_SPACE,
};

pub struct ${NAME_PASCAL}Driver;

impl Default for ${NAME_PASCAL}Driver {
    fn default() -> Self { Self }
}

impl Module for ${NAME_PASCAL}Driver {
    fn metadata(&self) -> &ModuleMetadata { &METADATA }
    
    fn init(&mut self, _ctx: &ModuleContext) -> Result<(), &'static str> {
        log::info!("Initializing ${NAME} driver");
        // TODO: Initialize hardware
        Ok(())
    }
    
    fn start(&mut self) -> Result<(), &'static str> {
        log::info!("Starting ${NAME} driver");
        Ok(())
    }
    
    fn stop(&mut self) -> Result<(), &'static str> {
        log::info!("Stopping ${NAME} driver");
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    
    fn dependencies(&self) -> &[ModuleDependency] { &[] }
    fn provides(&self) -> &[&str] { &["driver:${NAME}"] }
    fn save_state(&self) -> Option<alloc::vec::Vec<u8>> { None }
    fn restore_state(&mut self, _state: &[u8]) -> Result<(), &'static str> { Ok(()) }
}

define_module!(${NAME_PASCAL}Driver);
EOF

    # Replace placeholders
    local name_pascal=$(echo "${name}" | sed -r 's/(^|[-_])([a-z])/\U\2/g')
    local random_id=$(printf '%08X' $RANDOM$RANDOM)
    
    sed -i "s/\${NAME}/${name}/g" "${dir}/src/lib.rs"
    sed -i "s/\${NAME_PASCAL}/${name_pascal}/g" "${dir}/src/lib.rs"
    sed -i "s/\${RANDOM_ID}/${random_id}/g" "${dir}/src/lib.rs"
}

# =============================================================================
# Commands
# =============================================================================

cmd_add() {
    local module_type="$1"
    local module_name="$2"
    
    case "${module_type}" in
        scheduler)
            log_info "Creating scheduler module: ${module_name}"
            create_scheduler_module "${module_name}"
            ;;
        allocator)
            log_info "Creating allocator module: ${module_name}"
            create_allocator_module "${module_name}"
            ;;
        driver)
            log_info "Creating driver module: ${module_name}"
            create_driver_module "${module_name}"
            ;;
        *)
            log_error "Unknown module type: ${module_type}"
            log_info "Available types: scheduler, allocator, driver"
            exit 1
            ;;
    esac
    
    log_success "Module created at: ${MODULES_DIR}/${module_type}s/${module_name}"
    echo ""
    log_info "Don't forget to add the module to Cargo.toml workspace members!"
}

cmd_list() {
    print_header "Available Modules"
    
    for type_dir in "${MODULES_DIR}"/*/; do
        local type_name=$(basename "${type_dir}")
        echo -e "${HELIX_SECONDARY}${type_name}:${RESET}"
        
        for mod_dir in "${type_dir}"/*/; do
            if [[ -f "${mod_dir}/Cargo.toml" ]]; then
                local mod_name=$(basename "${mod_dir}")
                echo -e "  ${SYMBOL_MODULE} ${mod_name}"
            fi
        done
        echo ""
    done
}

cmd_info() {
    local module_name="$1"
    
    # Search for module
    for type_dir in "${MODULES_DIR}"/*/; do
        local mod_dir="${type_dir}/${module_name}"
        if [[ -f "${mod_dir}/Cargo.toml" ]]; then
            print_header "Module: ${module_name}"
            
            local type_name=$(basename "${type_dir}")
            print_keyvalue "Type" "${type_name}"
            print_keyvalue "Path" "${mod_dir}"
            
            # Extract version from Cargo.toml
            if grep -q "version.workspace" "${mod_dir}/Cargo.toml"; then
                print_keyvalue "Version" "workspace"
            fi
            
            # Count source files
            local src_count=$(find "${mod_dir}/src" -name "*.rs" | wc -l)
            print_keyvalue "Source files" "${src_count}"
            
            return 0
        fi
    done
    
    log_error "Module not found: ${module_name}"
    exit 1
}

# =============================================================================
# Usage
# =============================================================================

print_usage() {
    echo -e "${HELIX_PRIMARY}"
    cat << 'EOF'
    ██╗  ██╗███████╗██╗     ██╗██╗  ██╗
    ██║  ██║██╔════╝██║     ██║╚██╗██╔╝
    ███████║█████╗  ██║     ██║ ╚███╔╝ 
    ██╔══██║██╔══╝  ██║     ██║ ██╔██╗ 
    ██║  ██║███████╗███████╗██║██╔╝ ██╗
    ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═╝
EOF
    echo -e "${RESET}"
    echo -e "${DIM}    Module Manager${RESET}"
    echo ""
    
    echo -e "${BOLD}Usage:${RESET}"
    echo "  $0 <command> [args]"
    echo ""
    
    echo -e "${BOLD}Commands:${RESET}"
    echo "  add <type> <name>   Create a new module"
    echo "  list                List all modules"
    echo "  info <name>         Show module information"
    echo "  help                Show this help"
    echo ""
    
    echo -e "${BOLD}Module Types:${RESET}"
    echo "  scheduler           Scheduler module"
    echo "  allocator           Memory allocator module"
    echo "  driver              Device driver module"
    echo ""
    
    echo -e "${BOLD}Examples:${RESET}"
    echo "  $0 add scheduler my_scheduler"
    echo "  $0 add driver usb"
    echo "  $0 list"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    local command="${1:-help}"
    shift || true
    
    case "${command}" in
        add)
            if [[ $# -lt 2 ]]; then
                log_error "Usage: $0 add <type> <name>"
                exit 1
            fi
            cmd_add "$1" "$2"
            ;;
        list)
            cmd_list
            ;;
        info)
            if [[ $# -lt 1 ]]; then
                log_error "Usage: $0 info <name>"
                exit 1
            fi
            cmd_info "$1"
            ;;
        help|--help|-h)
            print_usage
            ;;
        *)
            log_error "Unknown command: ${command}"
            print_usage
            exit 1
            ;;
    esac
}

main "$@"
