use anyhow::{bail, Result};
use std::collections::HashMap;

use crate::ast::nodes::{Block, Expr, Function, Literal, Program, Statement};
use crate::typecheck::types::{TypeContext, TypeInfo, TypeError};

/// Type checker that validates and infers types in OtterLang programs
pub struct TypeChecker {
    errors: Vec<TypeError>,
    context: TypeContext,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut context = TypeContext::new();
        
        // Register built-in functions
        Self::register_builtins(&mut context);

        Self {
            errors: Vec::new(),
            context,
        }
    }

    /// Register all built-in functions in the type context
    fn register_builtins(context: &mut TypeContext) {
        // print function
        context.functions.insert(
            "print".to_string(),
            TypeInfo::Function {
                params: vec![TypeInfo::Str],
                return_type: Box::new(TypeInfo::Unit),
            },
        );

        // len functions
        context.functions.insert(
            "len".to_string(),
            TypeInfo::Function {
                params: vec![TypeInfo::Str],
                return_type: Box::new(TypeInfo::I64),
            },
        );
        
        // cap function
        context.functions.insert(
            "cap".to_string(),
            TypeInfo::Function {
                params: vec![TypeInfo::Str],
                return_type: Box::new(TypeInfo::I64),
            },
        );

        // panic function
        context.functions.insert(
            "panic".to_string(),
            TypeInfo::Function {
                params: vec![TypeInfo::Str],
                return_type: Box::new(TypeInfo::Unit),
            },
        );

        // Common math functions (from math module)
        for func_name in &["abs", "sqrt", "pow", "exp", "log", "sin", "cos", "tan", "floor", "ceil", "round"] {
            context.functions.insert(
                func_name.to_string(),
                TypeInfo::Function {
                    params: vec![TypeInfo::F64],
                    return_type: Box::new(TypeInfo::F64),
                },
            );
        }

        // min/max with two parameters
        for func_name in &["min", "max"] {
            context.functions.insert(
                func_name.to_string(),
                TypeInfo::Function {
                    params: vec![TypeInfo::F64, TypeInfo::F64],
                    return_type: Box::new(TypeInfo::F64),
                },
            );
        }
    }

    /// Type check a program
    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        // First pass: collect struct definitions and type aliases
        for statement in &program.statements {
            match statement {
                Statement::Struct { name, fields, generics, .. } => {
                    let mut field_types = HashMap::new();
                    for (field_name, field_ty) in fields {
                        let ty = TypeInfo::from(field_ty);
                        field_types.insert(field_name.clone(), ty);
                    }
                    
                    // Push generic parameters to context
                    for generic in generics {
                        self.context.push_generic(generic.clone());
                    }
                    
                    self.context.define_struct(name.clone(), field_types);
                    
                    // Pop generic parameters
                    for _ in generics {
                        self.context.pop_generic();
                    }
                }
                Statement::TypeAlias { name, target, .. } => {
                    let ty = TypeInfo::from(target);
                    self.context.define_type_alias(name.clone(), ty);
                }
                _ => {}
            }
        }

        // Second pass: collect function signatures
        for statement in &program.statements {
            if let Statement::Function(function) = statement {
                let sig = self.infer_function_signature(function);
                self.context
                    .functions
                    .insert(function.name.clone(), sig);
            }
        }

        // Third pass: type check function bodies and top-level statements
        for statement in &program.statements {
            match statement {
                Statement::Function(function) => {
                    self.check_function(function)?;
                }
                Statement::Let { .. } | Statement::Expr(_) => {
                    // Top-level let and expressions are allowed
                    self.check_statement(statement)?;
                }
                Statement::Struct { .. } | Statement::TypeAlias { .. } | Statement::Use { .. } => {
                    // These are handled in earlier passes
                }
                _ => {
                    self.errors.push(TypeError::new(format!(
                        "unexpected statement at top level: {:?}",
                        statement
                    )).with_hint("Only function definitions, let statements, and expressions are allowed at the top level".to_string()));
                }
            }
        }

        if !self.errors.is_empty() {
            let error_messages: Vec<String> = self
                .errors
                .iter()
                .map(|e| e.to_string())
                .collect();
            bail!("type checking failed:\n{}", error_messages.join("\n\n"));
        }

        Ok(())
    }

    /// Infer function signature from declaration
    fn infer_function_signature(&self, function: &Function) -> TypeInfo {
        let param_types: Vec<TypeInfo> = function
            .params
            .iter()
            .map(|p| {
                if let Some(ty) = &p.ty {
                    // Resolve type aliases
                    let type_info = TypeInfo::from(ty);
                    // Check if it's a type alias
                    if let TypeInfo::Generic { base, args } = &type_info {
                        if args.is_empty() {
                            // Could be a type alias or generic parameter
                            if let Some(aliased_type) = self.context.resolve_type_alias(base) {
                                aliased_type.clone()
                            } else {
                                type_info
                            }
                        } else {
                            type_info
                        }
                    } else {
                        type_info
                    }
                } else {
                    TypeInfo::Unknown
                }
            })
            .collect();

        let return_type = if let Some(ty) = &function.ret_ty {
            let type_info = TypeInfo::from(ty);
            // Resolve type aliases
            if let TypeInfo::Generic { base, args } = &type_info {
                if args.is_empty() {
                    if let Some(aliased_type) = self.context.resolve_type_alias(base) {
                        aliased_type.clone()
                    } else {
                        type_info
                    }
                } else {
                    type_info
                }
            } else {
                type_info
            }
        } else {
            TypeInfo::Unknown
        };

        TypeInfo::Function {
            params: param_types,
            return_type: Box::new(return_type),
        }
    }

    /// Type check a function
    fn check_function(&mut self, function: &Function) -> Result<()> {
        let mut fn_context = TypeContext::new();
        
        // Add function parameters to context
        for param in &function.params {
            let param_type = if let Some(ty) = &param.ty {
                TypeInfo::from(ty)
            } else {
                TypeInfo::Unknown
            };
            fn_context.insert_variable(param.name.clone(), param_type);
        }

        // Copy function signatures to inner context
        for (name, sig) in &self.context.functions {
            fn_context.functions.insert(name.clone(), sig.clone());
        }

        // Type check function body
        let old_context = std::mem::replace(&mut self.context, fn_context);
        let result = self.check_block(&function.body);
        self.context = old_context;

        result
    }

    /// Check function with generic type parameters
    /// This handles functions that have generic type parameters in their signature
    #[allow(dead_code)]
    fn check_function_with_generics(&mut self, function: &Function) -> Result<()> {
        // Extract generic type parameters from function signature
        let mut generic_params = Vec::new();
        
        // Check parameter types for generic parameters
        for param in &function.params {
            if let Some(ty) = &param.ty {
                self.extract_generic_params(ty, &mut generic_params);
            }
        }
        
        if let Some(ret_ty) = &function.ret_ty {
            self.extract_generic_params(ret_ty, &mut generic_params);
        }

        // Push generic parameters to context
        for param in &generic_params {
            self.context.push_generic(param.clone());
        }

        // Type check function body
        let result = self.check_function(function);

        // Pop generic parameters
        for _ in &generic_params {
            self.context.pop_generic();
        }

        result
    }

    /// Extract generic type parameter names from a type
    #[allow(dead_code)]
    fn extract_generic_params(&self, ty: &crate::ast::nodes::Type, params: &mut Vec<String>) {
        match ty {
            crate::ast::nodes::Type::Simple(name) => {
                // Check if this looks like a generic parameter (single uppercase letter or common generic names)
                // In OtterLang, generic parameters are typically single uppercase letters (T, U, etc.)
                if name.len() == 1 && name.chars().next().unwrap().is_uppercase() {
                    if !params.contains(name) {
                        params.push(name.clone());
                    }
                }
            }
            crate::ast::nodes::Type::Generic { base, args } => {
                // Check if base is a generic parameter
                if base.len() == 1 && base.chars().next().unwrap().is_uppercase() {
                    if !params.contains(base) {
                        params.push(base.clone());
                    }
                }
                // Recursively extract from type arguments
                for arg in args {
                    self.extract_generic_params(arg, params);
                }
            }
        }
    }

    /// Type check a block
    fn check_block(&mut self, block: &Block) -> Result<()> {
        for statement in &block.statements {
            self.check_statement(statement)?;
        }
        Ok(())
    }

    /// Type check a statement
    fn check_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Let { name, expr, .. } => {
                let expr_type = self.infer_expr_type(expr)?;
                self.context.insert_variable(name.clone(), expr_type);
            }
            Statement::Assignment { name, expr } => {
                let var_type = self
                    .context
                    .get_variable(name)
                    .ok_or_else(|| {
                        TypeError::new(format!("undefined variable: {}", name))
                            .with_hint(format!("did you mean to declare it with `let {}`?", name))
                            .with_help("Variables must be declared with `let` before they can be assigned".to_string())
                    })?
                    .clone();

                let expr_type = self.infer_expr_type(expr)?;
                if !expr_type.is_compatible_with(&var_type) {
                    self.errors.push(TypeError::new(format!(
                        "cannot assign {} to {} (expected {})",
                        expr_type.display_name(),
                        name,
                        var_type.display_name()
                    )).with_hint(format!("The variable `{}` is declared as `{}`, but you're trying to assign a value of type `{}`", name, var_type.display_name(), expr_type.display_name()))
                    .with_help("Make sure the types match or are compatible (e.g., i32 can be promoted to i64 or f64)".to_string()));
                }
            }
            Statement::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => {
                let cond_type = self.infer_expr_type(cond)?;
                if !cond_type.is_compatible_with(&TypeInfo::Bool) {
                    self.errors.push(TypeError::new(format!(
                        "if condition must be bool, got {}",
                        cond_type.display_name()
                    )));
                }

                self.check_block(then_block)?;
                for (_, block) in elif_blocks {
                    self.check_block(block)?;
                }
                if let Some(block) = else_block {
                    self.check_block(block)?;
                }
            }
            Statement::For { var, iterable, body } => {
                let _iter_type = self.infer_expr_type(iterable)?;
                // For now, assume iterable is a range or collection
                // TODO: Add proper iteration type checking
                self.context.insert_variable(var.clone(), TypeInfo::I32);
                self.check_block(body)?;
            }
            Statement::While { cond, body } => {
                let cond_type = self.infer_expr_type(cond)?;
                if !cond_type.is_compatible_with(&TypeInfo::Bool) {
                    self.errors.push(TypeError::new(format!(
                        "while condition must be bool, got {}",
                        cond_type.display_name()
                    )));
                }
                self.check_block(body)?;
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    let _expr_type = self.infer_expr_type(expr)?;
                    // TODO: Check return type matches function signature
                }
            }
            Statement::Function(_) => {
                // Functions are handled separately
            }
            Statement::Expr(expr) => {
                let _expr_type = self.infer_expr_type(expr)?;
                // Expression statements are allowed (e.g., function calls)
            }
            Statement::Break | Statement::Continue => {
                // These are handled by loop context
            }
            Statement::Use { .. } => {
                // Module imports are handled separately
            }
            Statement::Struct { .. } => {
                // Struct definitions are handled at the module level
            }
            Statement::TypeAlias { .. } => {
                // Type aliases are handled at the module level
            }
            Statement::Block(block) => {
                self.check_block(block)?;
            }
        }
        Ok(())
    }

    /// Infer the type of an expression
    pub fn infer_expr_type(&mut self, expr: &Expr) -> Result<TypeInfo> {
        match expr {
            Expr::Literal(lit) => Ok(match lit {
                Literal::Number(n) => {
                    // Check if number is whole (integer) or has decimal part
                    if n.fract() == 0.0 && *n <= i32::MAX as f64 && *n >= i32::MIN as f64 {
                        TypeInfo::I32
                    } else {
                        TypeInfo::F64
                    }
                }
                Literal::String(_) => TypeInfo::Str,
                Literal::Bool(_) => TypeInfo::Bool,
                Literal::Unit => TypeInfo::Unit,
            }),
            Expr::Identifier(name) => {
                self.context
                    .get_variable(name)
                    .cloned()
                    .ok_or_else(|| {
                        let err = TypeError::new(format!("undefined variable: {}", name))
                            .with_hint(format!("did you mean to declare it with `let {}`?", name))
                            .with_help("Variables must be declared before use".to_string());
                        anyhow::Error::from(err)
                    })
            }
            Expr::Binary { op, left, right } => {
                let left_type = self.infer_expr_type(left)?;
                let right_type = self.infer_expr_type(right)?;

                match op {
                    crate::ast::nodes::BinaryOp::Add
                    | crate::ast::nodes::BinaryOp::Sub
                    | crate::ast::nodes::BinaryOp::Mul
                    | crate::ast::nodes::BinaryOp::Div => {
                        // Numeric operations
                        match (&left_type, &right_type) {
                            (TypeInfo::F64, _) | (_, TypeInfo::F64) => Ok(TypeInfo::F64),
                            (TypeInfo::I64, _) | (_, TypeInfo::I64) => Ok(TypeInfo::I64),
                            (TypeInfo::I32, TypeInfo::I32) => Ok(TypeInfo::I32),
                            (TypeInfo::Str, TypeInfo::Str) if matches!(op, crate::ast::nodes::BinaryOp::Add) => Ok(TypeInfo::Str),
                            _ => {
                                self.errors.push(TypeError::new(format!(
                                    "cannot apply {} to {} and {}",
                                    format!("{:?}", op),
                                    left_type.display_name(),
                                    right_type.display_name()
                                )));
                                Ok(TypeInfo::Error)
                            }
                        }
                    }
                    crate::ast::nodes::BinaryOp::Eq
                    | crate::ast::nodes::BinaryOp::Ne
                    | crate::ast::nodes::BinaryOp::Lt
                    | crate::ast::nodes::BinaryOp::LtEq
                    | crate::ast::nodes::BinaryOp::Gt
                    | crate::ast::nodes::BinaryOp::GtEq => {
                        // Comparison operations return bool
                        if left_type.is_compatible_with(&right_type) {
                            Ok(TypeInfo::Bool)
                        } else {
                            self.errors.push(TypeError::new(format!(
                                "cannot compare {} and {}",
                                left_type.display_name(),
                                right_type.display_name()
                            )));
                            Ok(TypeInfo::Error)
                        }
                    }
                    crate::ast::nodes::BinaryOp::And | crate::ast::nodes::BinaryOp::Or => {
                        // Logical operations require bool operands
                        if left_type.is_compatible_with(&TypeInfo::Bool)
                            && right_type.is_compatible_with(&TypeInfo::Bool)
                        {
                            Ok(TypeInfo::Bool)
                        } else {
                            self.errors.push(TypeError::new(format!(
                                "logical operations require bool operands, got {} and {}",
                                left_type.display_name(),
                                right_type.display_name()
                            )));
                            Ok(TypeInfo::Error)
                        }
                    }
                    crate::ast::nodes::BinaryOp::Mod => {
                        // Modulo requires integer operands
                        match (&left_type, &right_type) {
                            (TypeInfo::I32, TypeInfo::I32) => Ok(TypeInfo::I32),
                            (TypeInfo::I64, TypeInfo::I64) => Ok(TypeInfo::I64),
                            _ => {
                                self.errors.push(TypeError::new(format!(
                                    "modulo requires integer operands, got {} and {}",
                                    left_type.display_name(),
                                    right_type.display_name()
                                )));
                                Ok(TypeInfo::Error)
                            }
                        }
                    }
                }
            }
            Expr::Unary { op, expr } => {
                let expr_type = self.infer_expr_type(expr)?;
                match op {
                    crate::ast::nodes::UnaryOp::Not => {
                        if expr_type.is_compatible_with(&TypeInfo::Bool) {
                            Ok(TypeInfo::Bool)
                        } else {
                            self.errors.push(TypeError::new(format!(
                                "not operator requires bool operand, got {}",
                                expr_type.display_name()
                            )));
                            Ok(TypeInfo::Error)
                        }
                    }
                    crate::ast::nodes::UnaryOp::Neg => {
                        if expr_type.is_compatible_with(&TypeInfo::I32)
                            || expr_type.is_compatible_with(&TypeInfo::I64)
                            || expr_type.is_compatible_with(&TypeInfo::F64)
                        {
                            Ok(expr_type)
                        } else {
                            self.errors.push(TypeError::new(format!(
                                "negation requires numeric operand, got {}",
                                expr_type.display_name()
                            )));
                            Ok(TypeInfo::Error)
                        }
                    }
                }
            }
            Expr::Call { func, args } => {
                let func_type = match func.as_ref() {
                    Expr::Identifier(name) => {
                        self.context
                            .get_function(name)
                            .cloned()
                            .ok_or_else(|| {
                                let err = TypeError::new(format!("undefined function: {}", name));
                                anyhow::Error::from(err)
                            })?
                    }
                    _ => {
                        self.errors.push(TypeError::new(
                            "function calls must use identifier".to_string(),
                        ));
                        return Ok(TypeInfo::Error);
                    }
                };

                match func_type {
                    TypeInfo::Function { params, return_type } => {
                        if args.len() != params.len() {
                            self.errors.push(TypeError::new(format!(
                                "function expects {} arguments, got {}",
                                params.len(),
                                args.len()
                            )).with_hint(format!("Check the function signature and provide the correct number of arguments"))
                            .with_help(format!("Expected: {} parameters, Got: {} arguments", params.len(), args.len())));
                            return Ok(TypeInfo::Error);
                        }

                        for (i, (arg, param_type)) in args.iter().zip(params.iter()).enumerate() {
                            let arg_type = self.infer_expr_type(arg)?;
                            if !arg_type.is_compatible_with(param_type) {
                                self.errors.push(TypeError::new(format!(
                                    "argument {} type mismatch: expected {}, got {}",
                                    i + 1,
                                    param_type.display_name(),
                                    arg_type.display_name()
                                )).with_hint(format!("Argument {} should be of type `{}`", i + 1, param_type.display_name()))
                                .with_help("Check the function signature and ensure argument types match".to_string()));
                            }
                        }

                        Ok(*return_type)
                    }
                    _ => {
                        self.errors.push(TypeError::new(format!(
                            "cannot call non-function type: {}",
                            func_type.display_name()
                        )).with_hint("Only functions can be called".to_string())
                        .with_help("Check that you're using the correct function name".to_string()));
                        Ok(TypeInfo::Error)
                    }
                }
            }
            Expr::Range { start, end } => {
                let start_type = self.infer_expr_type(start)?;
                let end_type = self.infer_expr_type(end)?;
                
                if start_type.is_compatible_with(&end_type) {
                    Ok(TypeInfo::Unknown) // Ranges are used for iteration
                } else {
                    self.errors.push(TypeError::new(format!(
                        "range bounds must have compatible types, got {} and {}",
                        start_type.display_name(),
                        end_type.display_name()
                    )));
                    Ok(TypeInfo::Error)
                }
            }
            Expr::If { cond, then_branch, else_branch } => {
                let cond_type = self.infer_expr_type(cond)?;
                if !cond_type.is_compatible_with(&TypeInfo::Bool) {
                    self.errors.push(TypeError::new(format!(
                        "if condition must be bool, got {}",
                        cond_type.display_name()
                    )));
                }

                let then_type = self.infer_expr_type(then_branch)?;
                if let Some(else_expr) = else_branch {
                    let else_type = self.infer_expr_type(else_expr)?;

                    if then_type.is_compatible_with(&else_type) {
                        Ok(then_type)
                    } else {
                        self.errors.push(TypeError::new(format!(
                            "if branches must have compatible types, got {} and {}",
                            then_type.display_name(),
                            else_type.display_name()
                        )));
                        Ok(TypeInfo::Error)
                    }
                } else {
                    // If expression without else returns unit
                    Ok(TypeInfo::Unit)
                }
            }
            Expr::Member { object, field } => {
                let object_type = self.infer_expr_type(object)?;
                match &object_type {
                    TypeInfo::Struct { fields, .. } => {
                        if let Some(field_type) = fields.get(field) {
                            Ok(field_type.clone())
                        } else {
                            self.errors.push(TypeError::new(format!(
                                "struct has no field '{}'",
                                field
                            )).with_hint(format!("Check the struct definition for available fields")));
                            Ok(TypeInfo::Error)
                        }
                    }
                    TypeInfo::Generic { base, .. } if base == "Array" || base == "List" => {
                        // Array/list member access - could be indexing or method call
                        // For now, assume it's a method/property access
                        Ok(TypeInfo::Unknown)
                    }
                    _ => {
                        self.errors.push(TypeError::new(format!(
                            "cannot access member '{}' on type {}",
                            field,
                            object_type.display_name()
                        )).with_hint(format!("Only struct types support member access")));
                        Ok(TypeInfo::Error)
                    }
                }
            }
            Expr::FString { parts } => {
                // F-strings always evaluate to strings
                // Type check all embedded expressions
                for part in parts {
                    if let crate::ast::nodes::FStringPart::Expr(expr) = part {
                        let _ = self.infer_expr_type(expr)?;
                        // We don't care about the type, just that it's valid
                    }
                }
                Ok(TypeInfo::Str)
            }
            Expr::Lambda { params, ret_ty, body } => {
                // Create a new context for the lambda
                let mut lambda_context = self.context.clone();
                
                // Add lambda parameters to context
                let mut param_types = Vec::new();
                for param in params {
                    let param_type = if let Some(ty) = &param.ty {
                        TypeInfo::from(ty)
                    } else {
                        TypeInfo::Unknown
                    };
                    param_types.push(param_type.clone());
                    lambda_context.insert_variable(param.name.clone(), param_type);
                }

                // Type check lambda body
                let old_context = std::mem::replace(&mut self.context, lambda_context);
                let body_result = self.check_block(body);
                self.context = old_context;

                if body_result.is_err() {
                    return Ok(TypeInfo::Error);
                }

                let return_type = if let Some(ty) = ret_ty {
                    TypeInfo::from(ty)
                } else {
                    // Try to infer return type from body
                    // For now, assume unit if no return statement
                    TypeInfo::Unit
                };

                Ok(TypeInfo::Function {
                    params: param_types,
                    return_type: Box::new(return_type),
                })
            }
            Expr::Array(elements) => {
                if elements.is_empty() {
                    // Empty array - can't infer element type
                    Ok(TypeInfo::Generic {
                        base: "Array".to_string(),
                        args: vec![TypeInfo::Unknown],
                    })
                } else {
                    // Infer common element type
                    let mut element_types: Vec<TypeInfo> = elements
                        .iter()
                        .map(|e| self.infer_expr_type(e))
                        .collect::<Result<Vec<_>>>()?;

                    // Find common type (use first element's type for now)
                    let common_type = element_types.remove(0);
                    
                    // Check all elements are compatible
                    for (i, elem_type) in element_types.iter().enumerate() {
                        if !common_type.is_compatible_with(elem_type) && !elem_type.is_compatible_with(&common_type) {
                            self.errors.push(TypeError::new(format!(
                                "array element {} has incompatible type: expected {}, got {}",
                                i + 1,
                                common_type.display_name(),
                                elem_type.display_name()
                            )).with_hint("All array elements must have compatible types".to_string()));
                        }
                    }

                    Ok(TypeInfo::Generic {
                        base: "Array".to_string(),
                        args: vec![common_type],
                    })
                }
            }
            Expr::Dict(entries) => {
                if entries.is_empty() {
                    // Empty dictionary - can't infer key/value types
                    Ok(TypeInfo::Generic {
                        base: "Dict".to_string(),
                        args: vec![TypeInfo::Unknown, TypeInfo::Unknown],
                    })
                } else {
                    // Infer common key and value types
                    let mut key_types = Vec::new();
                    let mut value_types = Vec::new();

                    for (key_expr, value_expr) in entries {
                        key_types.push(self.infer_expr_type(key_expr)?);
                        value_types.push(self.infer_expr_type(value_expr)?);
                    }

                    // Find common types
                    let common_key_type = key_types.remove(0);
                    let common_value_type = value_types.remove(0);

                    // Check all keys and values are compatible
                    for (i, key_type) in key_types.iter().enumerate() {
                        if !common_key_type.is_compatible_with(key_type) {
                            self.errors.push(TypeError::new(format!(
                                "dictionary key {} has incompatible type: expected {}, got {}",
                                i + 1,
                                common_key_type.display_name(),
                                key_type.display_name()
                            )).with_hint("All dictionary keys must have compatible types".to_string()));
                        }
                    }

                    for (i, value_type) in value_types.iter().enumerate() {
                        if !common_value_type.is_compatible_with(value_type) {
                            self.errors.push(TypeError::new(format!(
                                "dictionary value {} has incompatible type: expected {}, got {}",
                                i + 1,
                                common_value_type.display_name(),
                                value_type.display_name()
                            )).with_hint("All dictionary values must have compatible types".to_string()));
                        }
                    }

                    Ok(TypeInfo::Generic {
                        base: "Dict".to_string(),
                        args: vec![common_key_type, common_value_type],
                    })
                }
            }
            Expr::Match { value, arms } => {
                let _value_type = self.infer_expr_type(value)?;
                
                if arms.is_empty() {
                    self.errors.push(TypeError::new(
                        "match expression must have at least one arm".to_string()
                    ).with_hint("Add at least one pattern to match against".to_string()));
                    return Ok(TypeInfo::Error);
                }

                // Type check each arm
                let mut arm_types = Vec::new();
                for arm in arms {
                    // Check pattern matches value type (simplified - just check it's valid)
                    // TODO: More sophisticated pattern matching type checking
                    
                    // Check guard if present
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.infer_expr_type(guard)?;
                        if !guard_type.is_compatible_with(&TypeInfo::Bool) {
                            self.errors.push(TypeError::new(format!(
                                "match guard must be bool, got {}",
                                guard_type.display_name()
                            )));
                        }
                    }

                    // Get arm body type
                    let body_type = self.infer_expr_type(&arm.body)?;
                    arm_types.push(body_type);
                }

                // All arms must return compatible types
                let common_type = arm_types.remove(0);
                for (i, arm_type) in arm_types.iter().enumerate() {
                    if !common_type.is_compatible_with(arm_type) {
                        self.errors.push(TypeError::new(format!(
                            "match arm {} returns incompatible type: expected {}, got {}",
                            i + 1,
                            common_type.display_name(),
                            arm_type.display_name()
                        )).with_hint("All match arms must return compatible types".to_string()));
                    }
                }

                Ok(common_type)
            }
            Expr::Await(expr) => {
                // Await expects an async/awaitable type
                // For now, just type check the inner expression
                let inner_type = self.infer_expr_type(expr)?;
                
                // TODO: Check if inner_type is actually awaitable
                // For now, return the inner type
                Ok(inner_type)
            }
            Expr::Spawn(expr) => {
                // Spawn creates a task from an expression
                // Type check the inner expression
                let _inner_type = self.infer_expr_type(expr)?;
                
                // Spawn returns a task handle (for now, return Unknown)
                Ok(TypeInfo::Unknown)
            }
        }
    }

    /// Get collected errors
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::nodes::{BinaryOp, Expr, Literal};

    #[test]
    fn test_type_inference_literal() {
        let mut checker = TypeChecker::new();
        
        let expr = Expr::Literal(Literal::Number(42.0));
        let ty = checker.infer_expr_type(&expr).unwrap();
        assert_eq!(ty, TypeInfo::I32);

        let expr = Expr::Literal(Literal::Number(3.14));
        let ty = checker.infer_expr_type(&expr).unwrap();
        assert_eq!(ty, TypeInfo::F64);
    }

    #[test]
    fn test_type_inference_binary() {
        let mut checker = TypeChecker::new();
        
        let expr = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Literal(Literal::Number(1.0))),
            right: Box::new(Expr::Literal(Literal::Number(2.0))),
        };
        let ty = checker.infer_expr_type(&expr).unwrap();
        assert_eq!(ty, TypeInfo::I32);
    }
}

