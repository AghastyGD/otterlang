//! Memory management system for OtterLang
//!
//! Provides reference counting, garbage collection, and memory profiling

pub mod rc;
pub mod object;
pub mod gc;
pub mod profiler;
pub mod config;

pub use rc::{RcOtter, WeakOtter};
pub use object::OtterObject;
pub use gc::{GcStrategyTrait, MarkSweepGC, RcGC};
pub use profiler::{MemoryProfiler, AllocationInfo};
pub use config::{GcConfig, GcStrategy};

