use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use inkwell::builder::Builder;
use inkwell::context::Context as LlvmContext;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, FunctionType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

use crate::ast::nodes::{BinaryOp, Expr, Function, Literal, Program, Statement};
use crate::runtime::ffi;
use crate::runtime::symbol_registry::{FfiSignature, FfiType, SymbolRegistry};

pub struct CodegenOptions {
    pub emit_ir: bool,
    pub opt_level: CodegenOptLevel,
    pub enable_lto: bool,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            emit_ir: false,
            opt_level: CodegenOptLevel::Default,
            enable_lto: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CodegenOptLevel {
    None,
    Default,
    Aggressive,
}

impl From<CodegenOptLevel> for OptimizationLevel {
    fn from(value: CodegenOptLevel) -> Self {
        match value {
            CodegenOptLevel::None => OptimizationLevel::None,
            CodegenOptLevel::Default => OptimizationLevel::Default,
            CodegenOptLevel::Aggressive => OptimizationLevel::Aggressive,
        }
    }
}

pub struct BuildArtifact {
    pub binary: PathBuf,
    pub ir: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OtterType {
    Unit,
    Bool,
    I32,
    I64,
    F64,
    Str,
}

impl From<FfiType> for OtterType {
    fn from(value: FfiType) -> Self {
        match value {
            FfiType::Unit => OtterType::Unit,
            FfiType::Bool => OtterType::Bool,
            FfiType::I32 => OtterType::I32,
            FfiType::I64 => OtterType::I64,
            FfiType::F64 => OtterType::F64,
            FfiType::Str => OtterType::Str,
        }
    }
}

struct EvaluatedValue<'ctx> {
    ty: OtterType,
    value: Option<BasicValueEnum<'ctx>>,
}

impl<'ctx> EvaluatedValue<'ctx> {
    fn with_value(value: BasicValueEnum<'ctx>, ty: OtterType) -> Self {
        Self {
            ty,
            value: Some(value),
        }
    }
}

struct Variable<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: OtterType,
}

struct FunctionContext<'ctx> {
    variables: HashMap<String, Variable<'ctx>>,
}

impl<'ctx> FunctionContext<'ctx> {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&Variable<'ctx>> {
        self.variables.get(name)
    }

    fn insert(&mut self, name: String, variable: Variable<'ctx>) {
        self.variables.insert(name, variable);
    }
}

pub fn current_llvm_version() -> Option<String> {
    Some("15.0".to_string())
}

pub fn build_executable(
    program: &Program,
    output: &Path,
    options: &CodegenOptions,
) -> Result<BuildArtifact> {
    let context = LlvmContext::create();
    let module = context.create_module("otter");
    let builder = context.create_builder();
    let registry = ffi::bootstrap_stdlib();
    let mut compiler = Compiler::new(&context, module, builder, registry);

    compiler.lower_program(program)?;
    compiler
        .module
        .verify()
        .map_err(|e| anyhow!("LLVM module verification failed: {e}"))?;

    if options.emit_ir {
        // Ensure IR snapshot happens before LLVM potentially mutates the module during codegen.
        compiler.cached_ir = Some(compiler.module.print_to_string().to_string());
    }

    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| anyhow!("failed to initialise LLVM target: {e}"))?;

    let triple = TargetMachine::get_default_triple();
    compiler.module.set_triple(&triple);

    let target = Target::from_triple(&triple)
        .map_err(|e| anyhow!("failed to create target from triple: {e}"))?;

    let optimization: OptimizationLevel = options.opt_level.into();
    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            optimization,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| anyhow!("failed to create target machine"))?;

    compiler
        .module
        .set_data_layout(&target_machine.get_target_data().get_data_layout());

    compiler.run_default_passes(options.opt_level);

    let object_path = output.with_extension("o");
    target_machine
        .write_to_file(&compiler.module, FileType::Object, &object_path)
        .map_err(|e| {
            anyhow!(
                "failed to emit object file at {}: {e}",
                object_path.display()
            )
        })?;

            // Create a C runtime shim for the FFI functions
            let runtime_c = output.with_extension("runtime.c");
            let runtime_c_content = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <stdint.h>

void otter_std_io_print(const char* message) {
    if (message) {
        printf("%s", message);
        fflush(stdout);
    }
}

void otter_std_io_println(const char* message) {
    if (message) {
        printf("%s\n", message);
    } else {
        printf("\n");
    }
}

char* otter_std_io_read_line() {
    char* line = NULL;
    size_t len = 0;
    ssize_t read = getline(&line, &len, stdin);
    if (read == -1) {
        free(line);
        return NULL;
    }
    // Remove trailing newline
    if (read > 0 && line[read-1] == '\n') {
        line[read-1] = '\0';
    }
    return line;
}

void otter_std_io_free_string(char* ptr) {
    if (ptr) {
        free(ptr);
    }
}

int64_t otter_std_time_now_ms() {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (int64_t)tv.tv_sec * 1000 + tv.tv_usec / 1000;
}

// String formatting helpers
char* otter_format_float(double value) {
    char* buffer = (char*)malloc(64);
    if (buffer) {
        snprintf(buffer, 64, "%.10g", value);
    }
    return buffer;
}

char* otter_format_int(int64_t value) {
    char* buffer = (char*)malloc(32);
    if (buffer) {
        snprintf(buffer, 32, "%lld", (long long)value);
    }
    return buffer;
}

char* otter_concat_strings(const char* s1, const char* s2) {
    if (!s1 || !s2) return NULL;
    size_t len1 = strlen(s1);
    size_t len2 = strlen(s2);
    char* result = (char*)malloc(len1 + len2 + 1);
    if (result) {
        strcpy(result, s1);
        strcat(result, s2);
    }
    return result;
}

void otter_free_string_ptr(char* ptr) {
    if (ptr) {
        free(ptr);
    }
}
"#;
    fs::write(&runtime_c, runtime_c_content)
        .context("failed to write runtime C file")?;

    // Compile the runtime C file
    let runtime_o = output.with_extension("runtime.o");
    let cc_status = Command::new("cc")
        .arg("-c")
        .arg(&runtime_c)
        .arg("-o")
        .arg(&runtime_o)
        .status()
        .context("failed to compile runtime C file")?;

    if !cc_status.success() {
        bail!("failed to compile runtime C file");
    }

    // Link the object files together
    let mut cc = Command::new("cc");
    cc.arg(&object_path).arg(&runtime_o).arg("-o").arg(output);

    if options.enable_lto {
        cc.arg("-flto");
    }

    let status = cc.status().context("failed to invoke system linker (cc)")?;

    if !status.success() {
        bail!("linker invocation failed with status {status}");
    }

    // Clean up temporary files
    fs::remove_file(&runtime_c).ok();
    fs::remove_file(&runtime_o).ok();

    fs::remove_file(&object_path).ok();

    Ok(BuildArtifact {
        binary: output.to_path_buf(),
        ir: compiler.cached_ir.take(),
    })
}

struct Compiler<'ctx> {
    context: &'ctx LlvmContext,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    cached_ir: Option<String>,
    symbol_registry: &'static SymbolRegistry,
}

impl<'ctx> Compiler<'ctx> {
    fn new(
        context: &'ctx LlvmContext,
        module: Module<'ctx>,
        builder: Builder<'ctx>,
        symbol_registry: &'static SymbolRegistry,
    ) -> Self {
        Self {
            context,
            module,
            builder,
            cached_ir: None,
            symbol_registry,
        }
    }

    fn lower_program(&mut self, program: &Program) -> Result<()> {
        // Extract functions from statements
        let functions: Vec<&Function> = program.statements
            .iter()
            .filter_map(|stmt| match stmt {
                Statement::Function(func) => Some(func),
                _ => None,
            })
            .collect();

        if functions.is_empty() {
            bail!("program contains no functions");
        }

        // First, declare all functions (without bodies)
        for function in &functions {
            self.declare_function(function)?;
        }

        // Then, lower the bodies of all functions
        for function in &functions {
            self.lower_function_body(function)?;
        }

        if !functions.iter().any(|f| f.name == "main") {
            bail!("entry function `main` not found");
        }

        Ok(())
    }

    fn declare_function(&mut self, function: &Function) -> Result<FunctionValue<'ctx>> {
        // Determine parameter types
        let mut param_types = vec![];
        for param in &function.params {
            let ty = if let Some(ty_name) = &param.ty {
                self.type_from_name(ty_name)?
            } else {
                OtterType::F64 // Default to f64 if no type specified
            };
            param_types.push(self.basic_type(ty)?);
        }

        // Determine return type
        let ret_type = if let Some(ret_ty_name) = &function.ret_ty {
            self.type_from_name(ret_ty_name)?
        } else {
            OtterType::I32 // Default to i32 for compatibility
        };

        // Build function type
        let param_metadata: Vec<BasicMetadataTypeEnum> = param_types.iter().map(|&t| t.into()).collect();
        let fn_type = if ret_type == OtterType::Unit {
            self.context.void_type().fn_type(&param_metadata, false)
        } else {
            match self.basic_type(ret_type)? {
                BasicTypeEnum::IntType(t) => t.fn_type(&param_metadata, false),
                BasicTypeEnum::FloatType(t) => t.fn_type(&param_metadata, false),
                BasicTypeEnum::PointerType(t) => t.fn_type(&param_metadata, false),
                _ => bail!("unsupported return type"),
            }
        };

        Ok(self.module.add_function(&function.name, fn_type, None))
    }

    fn lower_function_body(&mut self, function: &Function) -> Result<()> {
        let llvm_fn = self.module.get_function(&function.name)
            .ok_or_else(|| anyhow!("function {} not declared", function.name))?;

        let entry = self.context.append_basic_block(llvm_fn, "entry");
        self.builder.position_at_end(entry);

        let mut ctx = FunctionContext::new();

        // Store parameters as local variables
        for (i, param) in function.params.iter().enumerate() {
            let param_value = llvm_fn.get_nth_param(i as u32).ok_or_else(|| {
                anyhow!("failed to get parameter {} for function {}", i, function.name)
            })?;

            let param_ty = if let Some(ty_name) = &param.ty {
                self.type_from_name(ty_name)?
            } else {
                OtterType::F64
            };

            let alloca = self.builder.build_alloca(self.basic_type(param_ty)?, &param.name);
            self.builder.build_store(alloca, param_value);
            ctx.insert(
                param.name.clone(),
                Variable {
                    ptr: alloca,
                    ty: param_ty,
                },
            );
        }

        for statement in &function.body.statements {
            self.lower_statement(statement, llvm_fn, &mut ctx)?;
        }

        // Add default return if needed
        if self
            .builder
            .get_insert_block()
            .and_then(|block| block.get_terminator())
            .is_none()
        {
            let ret_type = if let Some(ret_ty_name) = &function.ret_ty {
                self.type_from_name(ret_ty_name)?
            } else {
                OtterType::I32
            };

            if ret_type == OtterType::Unit {
                self.builder.build_return(None);
            } else {
                // Default return for non-void functions
                match ret_type {
                    OtterType::I32 => {
                        let val = self.context.i32_type().const_zero();
                        self.builder.build_return(Some(&val));
                    }
                    OtterType::I64 => {
                        let val = self.context.i64_type().const_zero();
                        self.builder.build_return(Some(&val));
                    }
                    OtterType::F64 => {
                        let val = self.context.f64_type().const_zero();
                        self.builder.build_return(Some(&val));
                    }
                    OtterType::Bool => {
                        let val = self.context.bool_type().const_zero();
                        self.builder.build_return(Some(&val));
                    }
                    _ => bail!("unsupported return type"),
                };
            }
        }

        Ok(())
    }

    fn type_from_name(&self, name: &str) -> Result<OtterType> {
        match name {
            "int" => Ok(OtterType::I64),
            "float" => Ok(OtterType::F64),
            "bool" => Ok(OtterType::Bool),
            "str" => Ok(OtterType::Str),
            _ => bail!("unknown type: {}", name),
        }
    }

    fn lower_statement(
        &mut self,
        statement: &Statement,
        _function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<()> {
        match statement {
            Statement::Expr(expr) => {
                // Just evaluate the expression (e.g., function calls like print())
                self.eval_expr(expr, ctx)?;
                Ok(())
            }
            Statement::Let { name, expr } => {
                let evaluated = self.eval_expr(expr, ctx)?;
                if evaluated.ty == OtterType::Unit {
                    bail!("cannot declare variable `{name}` with unit value");
                }

                let value = evaluated
                    .value
                    .clone()
                    .ok_or_else(|| anyhow!("expected value for `{name}`"))?;

                let ty = self.basic_type(evaluated.ty)?;
                let alloca = self.builder.build_alloca(ty, name);
                self.builder.build_store(alloca, value);
                
                ctx.insert(
                    name.clone(),
                    Variable {
                        ptr: alloca,
                        ty: evaluated.ty,
                    },
                );
                Ok(())
            }
            Statement::If { cond, then_block, elif_blocks, else_block } => {
                let cond_value = self.eval_expr(cond, ctx)?;
                if cond_value.ty != OtterType::Bool {
                    bail!("if condition must be a boolean, got {:?}", cond_value.ty);
                }

                let cond_bool = cond_value
                    .value
                    .ok_or_else(|| anyhow!("missing condition value"))?
                    .into_int_value();

                let then_bb = self.context.append_basic_block(_function, "then");
                let else_bb = self.context.append_basic_block(_function, "else");
                let merge_bb = self.context.append_basic_block(_function, "ifcont");

                self.builder.build_conditional_branch(cond_bool, then_bb, else_bb);

                // Then block
                self.builder.position_at_end(then_bb);
                for stmt in &then_block.statements {
                    self.lower_statement(stmt, _function, ctx)?;
                }
                if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
                    self.builder.build_unconditional_branch(merge_bb);
                }

                // Else/elif blocks
                self.builder.position_at_end(else_bb);
                if !elif_blocks.is_empty() {
                    // TODO: Handle elif blocks properly
                    // For now, just skip to else block or merge
                }
                if let Some(else_block) = else_block {
                    for stmt in &else_block.statements {
                        self.lower_statement(stmt, _function, ctx)?;
                    }
                }
                if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none() {
                    self.builder.build_unconditional_branch(merge_bb);
                }

                // Continue after if
                self.builder.position_at_end(merge_bb);
                Ok(())
            }
            Statement::For { var, iterable, body } => {
                // For now, we only support range expressions
                if let Expr::Range { start, end } = iterable {
                    let start_val = self.eval_expr(start, ctx)?;
                    let end_val = self.eval_expr(end, ctx)?;

                    // Support both int and float ranges, coercing int to float if needed
                    let (start_num, end_num, loop_ty, is_float): (BasicValueEnum, BasicValueEnum, OtterType, bool) = match (start_val.ty, end_val.ty) {
                        (OtterType::F64, OtterType::F64) => {
                            let start = start_val.value.ok_or_else(|| anyhow!("missing start value"))?.into_float_value();
                            let end = end_val.value.ok_or_else(|| anyhow!("missing end value"))?.into_float_value();
                            (start.into(), end.into(), OtterType::F64, true)
                        }
                        (OtterType::I64, OtterType::I64) | (OtterType::I32, OtterType::I32) => {
                            let start = start_val.value.ok_or_else(|| anyhow!("missing start value"))?.into_int_value();
                            let end = end_val.value.ok_or_else(|| anyhow!("missing end value"))?.into_int_value();
                            (start.into(), end.into(), OtterType::I64, false)
                        }
                        (OtterType::F64, OtterType::I64) | (OtterType::I64, OtterType::F64) => {
                            // Coerce to float
                            let start = if start_val.ty == OtterType::F64 {
                                start_val.value.ok_or_else(|| anyhow!("missing start value"))?.into_float_value()
                            } else {
                                let int_val = start_val.value.ok_or_else(|| anyhow!("missing start value"))?.into_int_value();
                                self.builder.build_signed_int_to_float(int_val, self.context.f64_type(), "start_to_float")
                            };
                            let end = if end_val.ty == OtterType::F64 {
                                end_val.value.ok_or_else(|| anyhow!("missing end value"))?.into_float_value()
                            } else {
                                let int_val = end_val.value.ok_or_else(|| anyhow!("missing end value"))?.into_int_value();
                                self.builder.build_signed_int_to_float(int_val, self.context.f64_type(), "end_to_float")
                            };
                            (start.into(), end.into(), OtterType::F64, true)
                        }
                        _ => bail!("for loop range start and end must be numeric types"),
                    };

                    // Create loop blocks
                    let loop_header = self.context.append_basic_block(_function, "loop_header");
                    let loop_body = self.context.append_basic_block(_function, "loop_body");
                    let loop_end = self.context.append_basic_block(_function, "loop_end");

                    // Allocate loop variable
                    let loop_var_type = self.basic_type(loop_ty)?;
                    let loop_var_ptr = self.builder.build_alloca(loop_var_type, var);
                    self.builder.build_store(loop_var_ptr, start_num);
                    ctx.insert(var.clone(), Variable { ptr: loop_var_ptr, ty: loop_ty });

                    // Jump to loop header
                    self.builder.build_unconditional_branch(loop_header);

                    // Loop header: check condition
                    self.builder.position_at_end(loop_header);
                    let current = self.builder.build_load(loop_var_type, loop_var_ptr, "current");
                    
                    let cond = if is_float {
                        self.builder.build_float_compare(
                            inkwell::FloatPredicate::OLT,
                            current.into_float_value(),
                            end_num.into_float_value(),
                            "loop_cond"
                        )
                    } else {
                        self.builder.build_int_compare(
                            inkwell::IntPredicate::SLT,
                            current.into_int_value(),
                            end_num.into_int_value(),
                            "loop_cond"
                        )
                    };
                    self.builder.build_conditional_branch(cond, loop_body, loop_end);

                    // Loop body
                    self.builder.position_at_end(loop_body);
                    for stmt in &body.statements {
                        self.lower_statement(stmt, _function, ctx)?;
                    }

                    // Increment loop variable
                    let current = self.builder.build_load(loop_var_type, loop_var_ptr, "current");
                    let next: BasicValueEnum = if is_float {
                        let one = self.context.f64_type().const_float(1.0);
                        self.builder.build_float_add(current.into_float_value(), one, "next").into()
                    } else {
                        let one = self.context.i64_type().const_int(1, false);
                        self.builder.build_int_add(current.into_int_value(), one, "next").into()
                    };
                    self.builder.build_store(loop_var_ptr, next);
                    self.builder.build_unconditional_branch(loop_header);

                    // Continue after loop
                    self.builder.position_at_end(loop_end);
                    Ok(())
                } else {
                    bail!("for loops currently only support range expressions");
                }
            }
            Statement::While { .. } => {
                // TODO: Implement while loops
                todo!("While loops not yet implemented")
            }
            Statement::Break => {
                // TODO: Implement break statements
                todo!("Break statements not yet implemented")
            }
            Statement::Continue => {
                // TODO: Implement continue statements
                todo!("Continue statements not yet implemented")
            }
            Statement::Function(_) => {
                // Functions are already lowered in lower_program
                Ok(())
            }
            Statement::Use { module: _, alias: _ } => {
                // For now, just register the module import
                // TODO: Implement proper module resolution
                // Module imports are handled at the expression evaluation level
                Ok(())
            }
            Statement::Block(_) => {
                // TODO: Implement block statements
                todo!("Block statements not yet implemented")
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    let evaluated = self.eval_expr(expr, ctx)?;
                    if let Some(value) = evaluated.value {
                        self.builder.build_return(Some(&value));
                    } else {
                        self.builder.build_return(None);
                    }
                } else {
                    self.builder.build_return(None);
                }
                Ok(())
            }
            Statement::Assignment { name, expr } => {
                let evaluated = self.eval_expr(expr, ctx)?;
                if evaluated.ty == OtterType::Unit {
                    bail!("cannot assign unit value to `{name}`");
                }

                let value = evaluated
                    .value
                    .clone()
                    .ok_or_else(|| anyhow!("expected value for assignment to `{name}`"))?;

                let ptr = if let Some(variable) = ctx.get(name) {
                    if variable.ty != evaluated.ty {
                        bail!(
                            "type mismatch assigning to `{name}`: existing {:?}, new {:?}",
                            variable.ty,
                            evaluated.ty
                        );
                    }
                    variable.ptr
                } else {
                    let ty = self.basic_type(evaluated.ty)?;
                    let alloca = self.builder.build_alloca(ty, name);
                    ctx.insert(
                        name.clone(),
                        Variable {
                            ptr: alloca,
                            ty: evaluated.ty,
                        },
                    );
                    alloca
                };

                self.builder.build_store(ptr, value);
                Ok(())
            }
        }
    }


    fn eval_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        match expr {
            Expr::Literal(literal) => {
                self.eval_literal(literal)
            }
            Expr::Identifier(name) => {
                if let Some(variable) = ctx.get(name) {
                    let ty = self.basic_type(variable.ty)?;
                    let loaded = self.builder.build_load(ty, variable.ptr, name);
                    Ok(EvaluatedValue::with_value(loaded, variable.ty))
                } else {
                    bail!("unknown identifier `{name}`");
                }
            }
            Expr::Binary { left, op, right } => {
                let mut left_value = self.eval_expr(left, ctx)?;
                let mut right_value = self.eval_expr(right, ctx)?;

                // Coerce int to float if needed
                if left_value.ty == OtterType::I64 && right_value.ty == OtterType::F64 {
                    let int_val = left_value.value.ok_or_else(|| anyhow!("missing value"))?.into_int_value();
                    let float_val = self.builder.build_signed_int_to_float(int_val, self.context.f64_type(), "inttofloat");
                    left_value = EvaluatedValue::with_value(float_val.into(), OtterType::F64);
                } else if left_value.ty == OtterType::F64 && right_value.ty == OtterType::I64 {
                    let int_val = right_value.value.ok_or_else(|| anyhow!("missing value"))?.into_int_value();
                    let float_val = self.builder.build_signed_int_to_float(int_val, self.context.f64_type(), "inttofloat");
                    right_value = EvaluatedValue::with_value(float_val.into(), OtterType::F64);
                }

                if left_value.ty != right_value.ty {
                    bail!("binary operation type mismatch: {:?} vs {:?}", left_value.ty, right_value.ty);
                }

                if left_value.ty != OtterType::F64 {
                    bail!("binary expressions currently support only f64 operands, got {:?}", left_value.ty);
                }

                let lhs = left_value
                    .value
                    .clone()
                    .ok_or_else(|| anyhow!("left operand missing value"))?
                    .into_float_value();
                let rhs = right_value
                    .value
                    .clone()
                    .ok_or_else(|| anyhow!("right operand missing value"))?
                    .into_float_value();

                let result = match op {
                    BinaryOp::Add => self.builder.build_float_add(lhs, rhs, "addtmp").into(),
                    BinaryOp::Sub => self.builder.build_float_sub(lhs, rhs, "subtmp").into(),
                    BinaryOp::Mul => self.builder.build_float_mul(lhs, rhs, "multmp").into(),
                    BinaryOp::Div => self.builder.build_float_div(lhs, rhs, "divtmp").into(),
                    BinaryOp::Mod => {
                        // Use LLVM's frem instruction for floating point modulo
                        self.builder.build_float_rem(lhs, rhs, "modtmp").into()
                    }
                    BinaryOp::Eq => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OEQ, lhs, rhs, "eqtmp");
                        return Ok(EvaluatedValue::with_value(cmp.into(), OtterType::Bool));
                    }
                    BinaryOp::Ne => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::ONE, lhs, rhs, "neqtmp");
                        return Ok(EvaluatedValue::with_value(cmp.into(), OtterType::Bool));
                    }
                    BinaryOp::Lt => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OLT, lhs, rhs, "lttmp");
                        return Ok(EvaluatedValue::with_value(cmp.into(), OtterType::Bool));
                    }
                    BinaryOp::Gt => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OGT, lhs, rhs, "gttmp");
                        return Ok(EvaluatedValue::with_value(cmp.into(), OtterType::Bool));
                    }
                    BinaryOp::LtEq => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OLE, lhs, rhs, "letmp");
                        return Ok(EvaluatedValue::with_value(cmp.into(), OtterType::Bool));
                    }
                    BinaryOp::GtEq => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OGE, lhs, rhs, "getmp");
                        return Ok(EvaluatedValue::with_value(cmp.into(), OtterType::Bool));
                    }
                    BinaryOp::And => {
                        // TODO: Implement logical and
                        todo!("Logical and not yet implemented")
                    }
                    BinaryOp::Or => {
                        // TODO: Implement logical or
                        todo!("Logical or not yet implemented")
                    }
                };

                Ok(EvaluatedValue::with_value(result, OtterType::F64))
            }
            Expr::Call { func, args } => {
                // Handle special cases first
                match &**func {
                    Expr::Identifier(name) => {
                        match name.as_str() {
                        "print" => {
                            if args.len() != 1 {
                                bail!("print expects exactly 1 argument");
                            }
                            let evaluated = self.eval_expr(&args[0], ctx)?;
                            if evaluated.ty != OtterType::Str {
                                bail!("print currently supports only string values, got {:?}", evaluated.ty);
                            }
                            let pointer = evaluated.value
                                .ok_or_else(|| anyhow!("print expected a pointer value"))?
                                .into_pointer_value();
                            // Use println instead of print to add newline
                            self.call_symbol("std.io.println", &[pointer.into()])?;
                            return Ok(EvaluatedValue { ty: OtterType::Unit, value: None });
                        }
                            "println" => {
                                if args.len() != 1 {
                                    bail!("println expects exactly 1 argument");
                                }
                                let evaluated = self.eval_expr(&args[0], ctx)?;
                                if evaluated.ty != OtterType::Str {
                                    bail!("println currently supports only string values, got {:?}", evaluated.ty);
                                }
                                let pointer = evaluated.value
                                    .ok_or_else(|| anyhow!("println expected a pointer value"))?
                                    .into_pointer_value();
                                self.call_symbol("std.io.println", &[pointer.into()])?;
                                return Ok(EvaluatedValue { ty: OtterType::Unit, value: None });
                            }
                            _ => {}
                        }
                    }
                    Expr::Member { object, field } => {
                        // Handle member access calls like time.now()
                        if let Expr::Identifier(module_name) = &**object {
                            let full_name = format!("{}:{}", module_name, field);
                            match full_name.as_str() {
                                "time:now" => {
                                    let function = self.declare_symbol_function("std.time.now")?;
                                    let call = self.builder.build_call(function, &[], "call_time_now");
                                    let value = call
                                        .try_as_basic_value()
                                        .left()
                                        .ok_or_else(|| anyhow!("call to `time.now` did not produce a value"))?;
                                    // time.now() returns i64 milliseconds, convert to f64
                                    let final_value = if value.is_int_value() {
                                        let int_val = value.into_int_value();
                                        self.builder.build_signed_int_to_float(int_val, self.context.f64_type(), "int_to_float").into()
                                    } else if value.is_float_value() {
                                        value
                                    } else {
                                        bail!("unexpected return type from time.now");
                                    };
                                    return Ok(EvaluatedValue::with_value(final_value, OtterType::F64));
                                }
                                _ => {
                                    // Try to resolve as a regular function
                                    if let Some(symbol) = self.symbol_registry.resolve(&full_name) {
                                        let function = self.declare_symbol_function(&full_name)?;
                                        let mut lowered_args = Vec::with_capacity(args.len());
                                        for arg_expr in args {
                                            let value = self.eval_expr(arg_expr, ctx)?;
                                            lowered_args.push(self.value_to_metadata(&value)?);
                                        }
                                        let call = self.builder.build_call(function, &lowered_args, &format!("call_{}", full_name.replace(':', "_").replace('.', "_")));
                                        let return_ty: OtterType = symbol.signature.result.into();
                                        let value = match return_ty {
                                            OtterType::Unit => None,
                                            _ => Some(call.try_as_basic_value().left().ok_or_else(|| {
                                                anyhow!("call to `{}` did not produce a return value", full_name)
                                            })?),
                                        };
                                        return Ok(EvaluatedValue { ty: return_ty, value });
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                self.eval_call(func, args, ctx)
            }
            Expr::Unary { op, expr } => {
                let val = self.eval_expr(expr, ctx)?;
                match op {
                    crate::ast::nodes::UnaryOp::Neg => {
                        if val.ty != OtterType::F64 {
                            bail!("negation only supported for floats currently");
                        }
                        let float_val = val.value.ok_or_else(|| anyhow!("missing value"))?.into_float_value();
                        let neg = self.builder.build_float_neg(float_val, "negtmp");
                        Ok(EvaluatedValue::with_value(neg.into(), OtterType::F64))
                    }
                    crate::ast::nodes::UnaryOp::Not => {
                        if val.ty != OtterType::Bool {
                            bail!("logical not only supported for booleans");
                        }
                        let bool_val = val.value.ok_or_else(|| anyhow!("missing value"))?.into_int_value();
                        let not = self.builder.build_not(bool_val, "nottmp");
                        Ok(EvaluatedValue::with_value(not.into(), OtterType::Bool))
                    }
                }
            }
            Expr::If { .. } => {
                // TODO: Implement if expressions
                todo!("If expressions not yet implemented")
            }
            Expr::Range { .. } => {
                // Range expressions are only used in for loops and are evaluated there
                bail!("Range expressions can only be used in for loops")
            }
            Expr::FString { parts } => {
                // Build the f-string at runtime by concatenating all parts
                let str_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                
                // Declare helper functions
                let format_float_fn = if let Some(f) = self.module.get_function("otter_format_float") {
                    f
                } else {
                    let fn_type = str_ptr_type.fn_type(&[self.context.f64_type().into()], false);
                    self.module.add_function("otter_format_float", fn_type, None)
                };
                
                let format_int_fn = if let Some(f) = self.module.get_function("otter_format_int") {
                    f
                } else {
                    let fn_type = str_ptr_type.fn_type(&[self.context.i64_type().into()], false);
                    self.module.add_function("otter_format_int", fn_type, None)
                };
                
                let concat_fn = if let Some(f) = self.module.get_function("otter_concat_strings") {
                    f
                } else {
                    let fn_type = str_ptr_type.fn_type(&[str_ptr_type.into(), str_ptr_type.into()], false);
                    self.module.add_function("otter_concat_strings", fn_type, None)
                };
                
                let free_fn = if let Some(f) = self.module.get_function("otter_free_string_ptr") {
                    f
                } else {
                    let fn_type = self.context.void_type().fn_type(&[str_ptr_type.into()], false);
                    self.module.add_function("otter_free_string_ptr", fn_type, None)
                };
                
                // Start with empty string
                let mut result_ptr = self.builder.build_global_string_ptr("", "empty_str").as_pointer_value();
                
                for part in parts {
                    match part {
                        crate::ast::nodes::FStringPart::Text(text) => {
                            let text_ptr = self.builder.build_global_string_ptr(text, "fstr_text").as_pointer_value();
                            let new_result = self.builder.build_call(
                                concat_fn,
                                &[result_ptr.into(), text_ptr.into()],
                                "concat"
                            ).try_as_basic_value().left().unwrap().into_pointer_value();
                            
                            result_ptr = new_result;
                        }
                        crate::ast::nodes::FStringPart::Expr(expr) => {
                            let evaluated = self.eval_expr(expr, ctx)?;
                            
                            // Format the value based on its type
                            let formatted_ptr = match evaluated.ty {
                                OtterType::F64 => {
                                    let float_val = evaluated.value
                                        .ok_or_else(|| anyhow!("f-string expression missing value"))?
                                        .into_float_value();
                                    self.builder.build_call(
                                        format_float_fn,
                                        &[float_val.into()],
                                        "format_float"
                                    ).try_as_basic_value().left().unwrap().into_pointer_value()
                                }
                                OtterType::I64 => {
                                    let int_val = evaluated.value
                                        .ok_or_else(|| anyhow!("f-string expression missing value"))?
                                        .into_int_value();
                                    self.builder.build_call(
                                        format_int_fn,
                                        &[int_val.into()],
                                        "format_int"
                                    ).try_as_basic_value().left().unwrap().into_pointer_value()
                                }
                                OtterType::Str => {
                                    evaluated.value
                                        .ok_or_else(|| anyhow!("f-string expression missing value"))?
                                        .into_pointer_value()
                                }
                                _ => bail!("unsupported type in f-string: {:?}", evaluated.ty),
                            };
                            
                            // Concatenate with result
                            let new_result = self.builder.build_call(
                                concat_fn,
                                &[result_ptr.into(), formatted_ptr.into()],
                                "concat"
                            ).try_as_basic_value().left().unwrap().into_pointer_value();
                            
                            // Free the formatted string if it was allocated
                            if matches!(evaluated.ty, OtterType::F64 | OtterType::I64) {
                                self.builder.build_call(free_fn, &[formatted_ptr.into()], "free");
                            }
                            
                            result_ptr = new_result;
                        }
                    }
                }
                
                Ok(EvaluatedValue::with_value(result_ptr.into(), OtterType::Str))
            }
            Expr::Member { object, field } => {
                // Handle standalone member access (not in a call)
                if let Expr::Identifier(module_name) = &**object {
                    let full_name = format!("{}:{}", module_name, field);
                    match full_name.as_str() {
                        "time:now" => {
                            let function = self.declare_symbol_function("std.time.now")?;
                            let call = self.builder.build_call(function, &[], "call_time_now");
                            let value = call
                                .try_as_basic_value()
                                .left()
                                .ok_or_else(|| anyhow!("call to `time.now` did not produce a value"))?;
                            // time.now() returns i64 milliseconds, convert to f64
                            let final_value = if value.is_int_value() {
                                let int_val = value.into_int_value();
                                self.builder.build_signed_int_to_float(int_val, self.context.f64_type(), "int_to_float").into()
                            } else if value.is_float_value() {
                                value
                            } else {
                                bail!("unexpected return type from time.now");
                            };
                            Ok(EvaluatedValue::with_value(final_value, OtterType::F64))
                        }
                        _ => {
                            if let Some(symbol) = self.symbol_registry.resolve(&full_name) {
                                let function = self.declare_symbol_function(&full_name)?;
                                let call = self.builder.build_call(function, &[], &format!("call_{}", full_name.replace(':', "_").replace('.', "_")));
                                let return_ty: OtterType = symbol.signature.result.into();
                                let value = match return_ty {
                                    OtterType::Unit => None,
                                    _ => Some(call.try_as_basic_value().left().ok_or_else(|| {
                                        anyhow!("call to `{}` did not produce a return value", full_name)
                                    })?),
                                };
                                Ok(EvaluatedValue { ty: return_ty, value })
                            } else {
                                bail!("unknown member access: {}.{}", module_name, field);
                            }
                        }
                    }
                } else {
                    bail!("member access currently only supports module.field syntax");
                }
            }
            Expr::Await(_) => {
                // TODO: Implement await expressions
                todo!("Await expressions not yet implemented")
            }
            Expr::Spawn(_) => {
                // TODO: Implement spawn expressions
                todo!("Spawn expressions not yet implemented")
            }
        }
    }

    fn eval_literal(&mut self, literal: &Literal) -> Result<EvaluatedValue<'ctx>> {
        match literal {
            Literal::String(value) => {
                let global = self.builder.build_global_string_ptr(value, "str");
                Ok(EvaluatedValue::with_value(
                    global.as_pointer_value().into(),
                    OtterType::Str,
                ))
            }
            Literal::Number(value) => {
                let float = self.context.f64_type().const_float(*value);
                Ok(EvaluatedValue::with_value(float.into(), OtterType::F64))
            }
            Literal::Bool(value) => {
                let bool_val = self.context.bool_type().const_int(*value as u64, false);
                Ok(EvaluatedValue::with_value(bool_val.into(), OtterType::Bool))
            }
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        match callee {
            Expr::Identifier(name) => {
                if let Some(symbol) = self.symbol_registry.resolve(name) {
                    if symbol.signature.params.len() != args.len() {
                        bail!(
                            "function `{name}` expected {} arguments but got {}",
                            symbol.signature.params.len(),
                            args.len()
                        );
                    }

                    let function = self.declare_symbol_function(name)?;
                    let mut lowered_args = Vec::with_capacity(args.len());

                    for (expr, expected) in args.iter().zip(symbol.signature.params.iter()) {
                        let value = self.eval_expr(expr, ctx)?;
                        let expected_ty: OtterType = expected.clone().into();
                        if value.ty != expected_ty {
                            bail!(
                                "argument type mismatch for `{name}`: expected {:?}, found {:?}",
                                expected_ty,
                                value.ty
                            );
                        }
                        lowered_args.push(self.value_to_metadata(&value)?);
                    }

                    let call_name = format!("call_{}", name.replace('.', "_"));
                    let call = self.builder.build_call(function, &lowered_args, &call_name);
                    let return_ty: OtterType = symbol.signature.result.into();
                    let value = match return_ty {
                        OtterType::Unit => None,
                        _ => Some(call.try_as_basic_value().left().ok_or_else(|| {
                            anyhow!("call to `{name}` did not produce a return value")
                        })?),
                    };
                    Ok(EvaluatedValue {
                        ty: return_ty,
                        value,
                    })
                } else if let Some(function) = self.module.get_function(name) {
                    // User-defined function call
                    let fn_type = function.get_type();
                    let param_types = fn_type.get_param_types();
                    
                    let mut lowered_args = Vec::with_capacity(args.len());
                    for (i, arg_expr) in args.iter().enumerate() {
                        let mut value = self.eval_expr(arg_expr, ctx)?;
                        
                        // Coerce argument type to match parameter type if needed
                        if i < param_types.len() {
                            let expected_llvm_ty = param_types[i];
                            
                            // Check if we need to convert f64 to i64 or vice versa
                            if expected_llvm_ty.is_int_type() && value.ty == OtterType::F64 {
                                // Convert f64 to i64
                                let float_val = value.value.ok_or_else(|| anyhow!("missing value"))?.into_float_value();
                                let int_val = self.builder.build_float_to_signed_int(float_val, self.context.i64_type(), "float_to_int");
                                value = EvaluatedValue::with_value(int_val.into(), OtterType::I64);
                            } else if expected_llvm_ty.is_float_type() && value.ty == OtterType::I64 {
                                // Convert i64 to f64
                                let int_val = value.value.ok_or_else(|| anyhow!("missing value"))?.into_int_value();
                                let float_val = self.builder.build_signed_int_to_float(int_val, self.context.f64_type(), "int_to_float");
                                value = EvaluatedValue::with_value(float_val.into(), OtterType::F64);
                            }
                        }
                        
                        lowered_args.push(self.value_to_metadata(&value)?);
                    }
                    
                    let call = self
                        .builder
                        .build_call(function, &lowered_args, &format!("call_{name}"));
                    
                    // Determine return type from function signature
                    let return_type = fn_type.get_return_type();
                    
                    if let Some(ret_ty) = return_type {
                        let value = call
                            .try_as_basic_value()
                            .left()
                            .ok_or_else(|| anyhow!("call to `{name}` did not produce a value"))?;
                        // Try to infer OtterType from LLVM type
                        let otter_ty = if ret_ty.is_float_type() {
                            OtterType::F64
                        } else if ret_ty.is_int_type() {
                            OtterType::I64
                        } else {
                            OtterType::I32
                        };
                        Ok(EvaluatedValue::with_value(value, otter_ty))
                    } else {
                        // Void function
                        Ok(EvaluatedValue { ty: OtterType::Unit, value: None })
                    }
                } else {
                    bail!("unknown function `{name}`");
                }
            }
            _ => bail!("only identifier calls are supported"),
        }
    }

    fn basic_type(&self, ty: OtterType) -> Result<BasicTypeEnum<'ctx>> {
        let ty = match ty {
            OtterType::Unit => bail!("unit type has no runtime representation"),
            OtterType::Bool => self.context.bool_type().into(),
            OtterType::I32 => self.context.i32_type().into(),
            OtterType::I64 => self.context.i64_type().into(),
            OtterType::F64 => self.context.f64_type().into(),
            OtterType::Str => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .into(),
        };
        Ok(ty)
    }

    fn value_to_metadata(
        &self,
        value: &EvaluatedValue<'ctx>,
    ) -> Result<BasicMetadataValueEnum<'ctx>> {
        let basic = value
            .value
            .clone()
            .ok_or_else(|| anyhow!("expected value for call argument"))?;
        Ok(basic.into())
    }

    fn call_symbol(&mut self, name: &str, args: &[BasicMetadataValueEnum<'ctx>]) -> Result<()> {
        let function = self.declare_symbol_function(name)?;
        let call_name = format!("call_{}", name.replace('.', "_"));
        self.builder.build_call(function, args, &call_name);
        Ok(())
    }

    fn declare_symbol_function(&mut self, name: &str) -> Result<FunctionValue<'ctx>> {
        let entry = self
            .symbol_registry
            .resolve(name)
            .ok_or_else(|| anyhow!("unresolved symbol `{name}`"))?;

        if let Some(function) = self.module.get_function(&entry.symbol) {
            return Ok(function);
        }

        let fn_type = self.ffi_signature_to_fn_type(&entry.signature)?;
        Ok(self.module.add_function(&entry.symbol, fn_type, None))
    }

    fn ffi_signature_to_fn_type(&self, signature: &FfiSignature) -> Result<FunctionType<'ctx>> {
        let params = self.ffi_param_types(&signature.params)?;
        let fn_type = match signature.result {
            FfiType::Unit => self.context.void_type().fn_type(&params, false),
            FfiType::Bool => self.context.bool_type().fn_type(&params, false),
            FfiType::I32 => self.context.i32_type().fn_type(&params, false),
            FfiType::I64 => self.context.i64_type().fn_type(&params, false),
            FfiType::F64 => self.context.f64_type().fn_type(&params, false),
            FfiType::Str => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .fn_type(&params, false),
        };
        Ok(fn_type)
    }

    fn ffi_param_types(&self, params: &[FfiType]) -> Result<Vec<BasicMetadataTypeEnum<'ctx>>> {
        params
            .iter()
            .map(|ty| self.ffi_type_to_basic(ty).map(Into::into))
            .collect()
    }

    fn ffi_type_to_basic(&self, ty: &FfiType) -> Result<BasicTypeEnum<'ctx>> {
        match ty {
            FfiType::Unit => bail!("unit type is not allowed in FFI parameter position"),
            FfiType::Bool => Ok(self.context.bool_type().into()),
            FfiType::I32 => Ok(self.context.i32_type().into()),
            FfiType::I64 => Ok(self.context.i64_type().into()),
            FfiType::F64 => Ok(self.context.f64_type().into()),
            FfiType::Str => Ok(self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .into()),
        }
    }

    fn run_default_passes(&self, level: CodegenOptLevel) {
        if matches!(level, CodegenOptLevel::None) {
            return;
        }

        let pass_manager = PassManager::create(());
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_cfg_simplification_pass();
        pass_manager.add_instruction_simplify_pass();
        pass_manager.run_on(&self.module);
    }
}
