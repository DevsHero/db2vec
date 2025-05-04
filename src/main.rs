pub mod db;
pub mod parser;
pub mod embedding;
pub mod cli;
pub mod util;
pub mod workflow;
use clap::Parser;
use cli::Args;
use db::select_database;
use dotenvy::dotenv;

use log::{ info, error };
use crate::util::{ read_file_and_detect_format, logo };
use parser::parse_database_export;
use workflow::execute_migration_workflow;

fn main() -> Result<(), db::DbError> {
    logo();
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off")).init();
    let args = Args::parse();
    let file_path = args.data_file.clone();
    util::init_thread_pool(args.num_threads);

    let (content, format) = match read_file_and_detect_format(&file_path) {
        Ok(result) => result,
        Err(e) => {
            let err_msg = format!("Error reading file '{}': {}", file_path, e);
            error!("{}", err_msg);
            return Err(err_msg.into());
        }
    };

    let records = match parse_database_export(&content, &format, &args) {
        Ok(recs) => recs,
        Err(e) => {
            let err_msg = format!("Error parsing database export: {}", e);
            error!("{}", err_msg);
            return Err(err_msg.into());
        }
    };

    let database = select_database(&args)?;
    match execute_migration_workflow(records, &*database, &args) {
        Ok(stats) => {
            info!(
                "Migration successful: {} records processed in {:.2} seconds",
                stats.processed_records,
                stats.elapsed_seconds
            );
            Ok(())
        }
        Err(e) => {
            error!("Migration failed: {}", e);
            Err(e)
        }
    }
}