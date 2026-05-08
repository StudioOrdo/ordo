use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use ordo_daemon::health::{build_health_report, build_readiness_report};
use ordo_daemon::schema::init_database;
use ordo_daemon::server::serve;

#[derive(Parser)]
#[command(version, about = "Ordo appliance daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "health-json")]
    HealthJson,
    #[command(name = "ready-json")]
    ReadyJson {
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
    },
    #[command(name = "init-db")]
    InitDb {
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
    },
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 17760)]
        port: u16,
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::HealthJson => {
            println!("{}", serde_json::to_string_pretty(&build_health_report())?);
        }
        Commands::ReadyJson { db_path } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_readiness_report(&db_path))?
            );
        }
        Commands::InitDb { db_path } => {
            init_database(&db_path)?;
            println!("{}", serde_json::json!({ "ok": true, "dbPath": db_path }));
        }
        Commands::Serve {
            host,
            port,
            db_path,
        } => serve(host, port, db_path).await?,
    }

    Ok(())
}
