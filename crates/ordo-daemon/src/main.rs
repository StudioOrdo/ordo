use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use ordo_daemon::backups::{
    create_backup, list_backup_restore_jobs, run_restore_preflight, RestorePreflightRequest,
};
use ordo_daemon::briefs::{generate_system_brief, latest_system_brief, LatestBriefResponse};
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
    #[command(name = "latest-system-brief-json")]
    LatestSystemBriefJson {
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
    },
    #[command(name = "generate-system-brief-json")]
    GenerateSystemBriefJson {
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
    },
    #[command(name = "create-backup-json")]
    CreateBackupJson {
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
    },
    #[command(name = "list-backups-json")]
    ListBackupsJson {
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
    },
    #[command(name = "restore-preflight-json")]
    RestorePreflightJson {
        #[arg(long, env = "ORDO_DB_PATH", default_value = ".data/local.db")]
        db_path: PathBuf,
        #[arg(long)]
        backup_id: String,
        #[arg(long)]
        confirmation: String,
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
        Commands::LatestSystemBriefJson { db_path } => {
            init_database(&db_path)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&LatestBriefResponse {
                    brief: latest_system_brief(&db_path)?
                })?
            );
        }
        Commands::GenerateSystemBriefJson { db_path } => {
            init_database(&db_path)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&LatestBriefResponse {
                    brief: Some(generate_system_brief(&db_path, "cli", None)?)
                })?
            );
        }
        Commands::CreateBackupJson { db_path } => {
            init_database(&db_path)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&create_backup(&db_path, "cli", None)?)?
            );
        }
        Commands::ListBackupsJson { db_path } => {
            init_database(&db_path)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&list_backup_restore_jobs(&db_path)?)?
            );
        }
        Commands::RestorePreflightJson {
            db_path,
            backup_id,
            confirmation,
        } => {
            init_database(&db_path)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&run_restore_preflight(
                    &db_path,
                    RestorePreflightRequest {
                        backup_id,
                        confirmation,
                    },
                    "cli",
                    None,
                )?)?
            );
        }
        Commands::Serve {
            host,
            port,
            db_path,
        } => serve(host, port, db_path).await?,
    }

    Ok(())
}
