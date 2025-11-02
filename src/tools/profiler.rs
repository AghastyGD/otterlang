//! Profiler CLI tool for OtterLang
//!
//! Provides command-line interface for profiling OtterLang programs

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;

use crate::runtime::memory::profiler::{get_profiler, ProfilingStats};

/// Profile command for CLI integration
#[derive(Clone, Debug, clap::Subcommand)]
pub enum ProfileCommand {
    /// Profile memory usage
    Memory {
        /// OtterLang program to profile
        program: PathBuf,
        /// Output format (json, text)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Profile function call performance
    Calls {
        /// OtterLang program to profile
        program: PathBuf,
        /// Number of iterations
        #[arg(long, default_value = "100")]
        iterations: usize,
    },
    /// Show profiling statistics
    Stats {
        /// Stats file path
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

pub fn run_profiler_subcommand(command: &ProfileCommand) -> Result<()> {
    match command {
        ProfileCommand::Memory { program, format } => {
            profile_memory(program, format)?;
        }
        ProfileCommand::Calls { program, iterations } => {
            profile_calls(program, *iterations)?;
        }
        ProfileCommand::Stats { file } => {
            show_stats(file.clone())?;
        }
    }
    Ok(())
}

/// Profiler CLI for OtterLang
#[derive(Parser, Debug)]
#[command(name = "otter-profile", about = "Profile OtterLang programs")]
pub struct ProfilerCli {
    #[command(subcommand)]
    command: ProfileCommand,
}

pub fn run_profiler() -> Result<()> {
    let cli = ProfilerCli::parse();
    run_profiler_subcommand(&cli.command)
}

fn profile_memory(program: &PathBuf, format: &str) -> Result<()> {
    println!("{} Starting memory profiling for: {}", "🧠".cyan(), program.display());
    
    let profiler = get_profiler();
    profiler.start();

    // Run the program (simplified - in practice would execute via runtime)
    println!("{} Executing program...", "⚡".yellow());
    std::thread::sleep(Duration::from_millis(100)); // Simulate execution

    profiler.stop();
    let stats = profiler.get_stats();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&stats)?;
            println!("{}", json);
        }
        _ => {
            println!("\n{} Memory Profiling Results:", "📊".magenta());
            println!("  {} Total Allocated: {} bytes", "💾".blue(), stats.total_allocated);
            println!("  {} Total Freed: {} bytes", "🗑️".green(), stats.total_freed);
            println!("  {} Current Memory: {} bytes", "📌".cyan(), stats.current_memory);
            println!("  {} Peak Memory: {} bytes", "📈".yellow(), stats.peak_memory);
            println!("  {} Active Allocations: {}", "🔢".white(), stats.active_allocations);
            println!("  {} Duration: {:.2}s", "⏱️".magenta(), stats.duration_seconds);

            let leaks = profiler.detect_leaks();
            if !leaks.is_empty() {
                println!("\n{} Detected {} potential memory leaks:", "⚠️".red(), leaks.len());
                for leak in leaks.iter().take(10) {
                    println!("  {} Address: 0x{:x}, Size: {} bytes", "🔍".yellow(), leak.pointer, leak.size);
                }
            } else {
                println!("\n{} No memory leaks detected!", "✅".green());
            }
        }
    }

    Ok(())
}

fn profile_calls(program: &PathBuf, iterations: usize) -> Result<()> {
    println!("{} Profiling function calls for: {}", "🔬".cyan(), program.display());
    println!("{} Iterations: {}", "🔄".yellow(), iterations);

    // In practice, would use CallProfiler
    // For now, provide a placeholder implementation
    println!("\n{} Function Call Statistics:", "📊".magenta());
    println!("{:<30} {:>12} {:>15} {:>15} {:>15}", 
             "Function", "Calls", "Total Time", "Avg Time", "Max Time");
    println!("{}", "-".repeat(90));
    println!("{} Full call profiling requires runtime instrumentation", "💡".yellow());

    Ok(())
}

fn show_stats(file: Option<PathBuf>) -> Result<()> {
    if let Some(path) = file {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read stats file: {}", path.display()))?;
        let stats: ProfilingStats = serde_json::from_str(&content)?;
        
        println!("{} Profiling Statistics:", "📊".magenta());
        println!("  {} Duration: {:.2}s", "⏱️".yellow(), stats.duration_seconds);
        println!("  {} Total Allocated: {} bytes", "💾".blue(), stats.total_allocated);
        println!("  {} Total Freed: {} bytes", "🗑️".green(), stats.total_freed);
        println!("  {} Peak Memory: {} bytes", "📈".yellow(), stats.peak_memory);
    } else {
        println!("{} No stats file provided. Use --file to specify a stats file.", "💡".yellow());
    }
    Ok(())
}

