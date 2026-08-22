use sea_orm_migration::cli;
use shipr::app::migrations::Migrator;

#[tokio::main]
async fn main() {
    cli::run_cli(Migrator).await;
}
