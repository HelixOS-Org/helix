//! # MAGMA Vulkan Implementation
//!
//! Vulkan 1.3 driver implementation for NVIDIA GPUs.
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                        MAGMA Vulkan ICD                           │
//! │                                                                   │
//! │  ┌─────────────────────────────────────────────────────────────┐  │
//! │  │                  Vulkan Entry Points                        │  │
//! │  │  vkCreateInstance, vkCreateDevice, vkCmdDraw, etc.         │  │
//! │  └─────────────────────────────────────────────────────────────┘  │
//! │                              │                                    │
//! │  ┌──────────────┐  ┌────────┴────────┐  ┌───────────────────┐   │
//! │  │   Instance   │  │     Device      │  │   Command Pool    │   │
//! │  │   (VkInst)   │  │   (VkDevice)    │  │   (VkCmdPool)     │   │
//! │  └──────────────┘  └─────────────────┘  └───────────────────┘   │
//! │                              │                                    │
//! │  ┌─────────────────────────────────────────────────────────────┐  │
//! │  │                    MAGMA Core Layer                         │  │
//! │  │           magma-core, magma-cmd, magma-mem                  │  │
//! │  └─────────────────────────────────────────────────────────────┘  │
//! └───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Vulkan 1.3 Features
//!
//! - Dynamic rendering
//! - Synchronization2
//! - Extended dynamic state
//! - Private data
//! - Timeline semaphores
//! - Buffer device address

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod device;
pub mod entry;
pub mod extensions;
pub mod instance;
pub mod result;
pub mod types;

// Re-exports
pub use device::{
    MagmaDevice, MagmaPhysicalDevice, VkDeviceCreateInfo, VkDeviceQueueCreateInfo,
    VkMemoryHeap, VkMemoryHeapFlags, VkMemoryPropertyFlags, VkMemoryType,
    VkPhysicalDeviceLimits, VkPhysicalDeviceMemoryProperties, VkPhysicalDeviceProperties,
    VkQueueFamilyProperties, VkQueueFlags,
};
pub use entry::{
    PfnVkGetInstanceProcAddr, PfnVkVoidFunction, VkApplicationInfo, VkGlobalDispatch,
    VkInstanceCreateInfo,
};
pub use extensions::{
    DeviceExtensions, InstanceExtensions, VkExtensionProperties, VkLayerProperties,
    VkPhysicalDeviceFeatures, VkPhysicalDeviceVulkan13Features,
};
pub use instance::{PhysicalDeviceInfo, VkInstance, VkPhysicalDeviceType};
pub use result::{VkError, VkResult, VkSuccess};
pub use types::*;
