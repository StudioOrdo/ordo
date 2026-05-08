use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;

use crate::schema::REQUIRED_TABLES;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub schema_version: String,
    pub service: String,
    pub status: String,
    pub mode: String,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessReport {
    pub schema_version: String,
    pub service: String,
    pub status: String,
    pub checks: Vec<HealthCheck>,
}

pub fn build_health_report() -> HealthReport {
    HealthReport {
        schema_version: "1".to_string(),
        service: "ordo-daemon".to_string(),
        status: "ok".to_string(),
        mode: "phase_1_appliance_spine".to_string(),
        checks: vec![HealthCheck {
            name: "daemon".to_string(),
            status: "ok".to_string(),
            detail: "Daemon process is responding.".to_string(),
        }],
    }
}

pub fn build_readiness_report(db_path: &Path) -> ReadinessReport {
    let mut checks = Vec::new();
    let mut ready = true;

    if !db_path.exists() {
        ready = false;
        checks.push(HealthCheck {
            name: "sqlite".to_string(),
            status: "error".to_string(),
            detail: format!("Database does not exist at {}.", db_path.display()),
        });
    } else {
        match Connection::open(db_path).and_then(|connection| count_required_tables(&connection)) {
            Ok(count) if count == REQUIRED_TABLES.len() => checks.push(HealthCheck {
                name: "sqlite".to_string(),
                status: "ok".to_string(),
                detail: "Required tables are present.".to_string(),
            }),
            Ok(count) => {
                ready = false;
                checks.push(HealthCheck {
                    name: "sqlite".to_string(),
                    status: "error".to_string(),
                    detail: format!(
                        "Database has {count} of {} required tables.",
                        REQUIRED_TABLES.len()
                    ),
                });
            }
            Err(error) => {
                ready = false;
                checks.push(HealthCheck {
                    name: "sqlite".to_string(),
                    status: "error".to_string(),
                    detail: error.to_string(),
                });
            }
        }
    }

    ReadinessReport {
        schema_version: "1".to_string(),
        service: "ordo-daemon".to_string(),
        status: if ready { "ready" } else { "not_ready" }.to_string(),
        checks,
    }
}

fn count_required_tables(connection: &Connection) -> rusqlite::Result<usize> {
    let mut count = 0;
    for table_name in REQUIRED_TABLES {
        let exists: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )?;
        if exists == 1 {
            count += 1;
        }
    }
    Ok(count)
}
