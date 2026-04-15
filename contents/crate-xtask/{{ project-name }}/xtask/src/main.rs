//! Build automation tasks for {{ project-title }}
//!
//! Usage: cargo xtask <command>

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for {{ project-name }}")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the binary to ~/.cargo/bin
    Install,

    /// Run the application with arguments
    Run {
        /// Arguments to pass to the application
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Run unit tests
    TestUt,

    /// Run integration tests
    TestIt,

    /// Run all tests (unit + integration)
    TestAll,

    /// Build release binary
    Build,

    /// Check code without building
    Check,

    /// Run clippy lints (deny warnings)
    Clippy,

    /// Format code
    Fmt {
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },

    /// Sweep stale build artifacts from target/
    Sweep,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install => {
            cargo(&["install", "--path", "crates/{{ project-name }}-bin"])?;
        }

        Commands::Run { args } => {
            let mut cmd_args = vec!["run", "--package", "{{ project-name }}-bin", "--"];
            cmd_args.extend(args.iter().map(|s| s.as_str()));
            cargo(&cmd_args)?;
        }

        Commands::TestUt => {
            sweep()?;
            cargo(&["test", "--workspace", "--lib"])?;
        }

        Commands::TestIt => {
            sweep()?;
            cargo(&["test", "--workspace", "--tests"])?;
        }

        Commands::TestAll => {
            sweep()?;
            cargo(&["test", "--workspace"])?;
        }

        Commands::Build => {
            sweep()?;
            cargo(&["build", "--release"])?;
        }

        Commands::Check => {
            sweep()?;
            cargo(&["check", "--workspace", "--all-targets"])?;
        }

        Commands::Clippy => {
            sweep()?;
            cargo(&["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
        }

        Commands::Fmt { check } => {
            if check {
                cargo(&["fmt", "--all", "--", "--check"])?;
            } else {
                cargo(&["fmt", "--all"])?;
            }
        }

        Commands::Sweep => {
            sweep()?;
        }
    }

    Ok(())
}

fn cargo(args: &[&str]) -> Result<()> {
    println!("cargo {}", args.join(" "));

    let status = Command::new("cargo")
        .args(args)
        .status()?;

    if !status.success() {
        anyhow::bail!("cargo command failed with status: {}", status);
    }

    Ok(())
}

/// Sweep build artifacts older than 7 days. Installs cargo-sweep if missing.
fn sweep() -> Result<()> {
    ensure_cargo_sweep()?;
    println!("==> Sweeping stale artifacts (>7 days)...");
    let status = Command::new("cargo")
        .args(["sweep", "--time", "7"])
        .status()
        .context("failed to run cargo sweep")?;
    if !status.success() {
        eprintln!("    Warning: cargo sweep failed, continuing anyway");
    }
    Ok(())
}

/// Install cargo-sweep if it isn't already present.
fn ensure_cargo_sweep() -> Result<()> {
    let cargo_bin = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cargo/bin/cargo-sweep");

    if cargo_bin.exists() {
        return Ok(());
    }

    println!("==> Installing cargo-sweep...");
    let status = Command::new("cargo")
        .args(["install", "cargo-sweep"])
        .status()
        .context("failed to install cargo-sweep")?;
    if !status.success() {
        anyhow::bail!("cargo install cargo-sweep failed (exit {})", status);
    }
    Ok(())
}
