//! Module system for OtterLang
//!
//! Handles module resolution, loading, and dependency tracking for .otter files

pub mod resolver;
pub mod loader;
pub mod processor;

pub use resolver::{ModulePath, ModuleResolver, DependencyGraph};
pub use loader::{Module, ModuleExports, ModuleLoader};
pub use processor::ModuleProcessor;

