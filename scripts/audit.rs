use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "audit")]
#[command(about = "Grainlify Contract Audit Tool", long_about = None)]
struct Cli {
    /// The contract ID to audit
    #[arg(long)]
    contract: String,

    /// The network to use (e.g., testnet, futurenet)
    #[arg(long, default_value = "testnet")]
    network: String,

    /// Optional specific bounty ID to audit
    #[arg(long)]
    bounty: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    println!("Auditing Contract: {}", cli.contract);
    println!("Network: {}", cli.network);

    let mut cmd = Command::new("stellar");
    cmd.arg("contract")
       .arg("invoke")
       .arg("--id")
       .arg(&cli.contract)
       .arg("--network")
       .arg(&cli.network)
       .arg("--")
       .arg("audit_state");

    if let Some(bid) = cli.bounty {
        println!("Auditing specific bounty: {}", bid);
        cmd.arg("--bounty_id").arg(bid);
    } else {
        println!("Performing Global Audit...");
    }
    
    match cmd.status() {
        Ok(status) => {
            if status.success() {
                println!("Audit command executed successfully.");
            } else {
                eprintln!("Audit command failed.");
            }
        }
        Err(e) => {
            eprintln!("Failed to execute stellar cli: {}", e);
            eprintln!("Ensure 'stellar' CLI is installed and in your PATH.");
        }
    }
}
