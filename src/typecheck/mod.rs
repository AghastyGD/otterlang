//! Type checking system for OtterLang
//!
//! Provides type inference, validation, and error reporting

pub mod types;
pub mod checker;

pub use types::{TypeContext, TypeInfo, TypeError};
pub use checker::TypeChecker;

