
use clap::{Parser, Subcommand};

use crate::{
    app_errors::AppError,
    core::{
        app_types::DeployReq,
        controller::{
            cli::{deploy::deploy, listen::listen, serve::serve},
            storage::s3::S3Service,
            vm::{heartbeat_store::HeartbeatStore, id_allocator::IdAllocator, vm_pool::VmPool},
        },
    },
    shared::github_app::GithubApp,
};

#[derive(Parser)]
#[command(name = "shipr")]
#[command(about = "Shipr CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Deploy {
        #[arg(long)]
        url: String,

        #[arg(long)]
        install: Vec<String>,

        #[arg(long)]
        build: Vec<String>,

        #[arg(long)]
        run: Vec<String>,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        home_dir: String,

        #[arg(long)]
        dist_dir: String,
    },
    Serve,
    Listen,
    Test,
}

pub async fn cli(
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    s3_service: S3Service,
    heartbeat_store: HeartbeatStore,
) -> Result<(), AppError> {
    let args = Cli::parse();

    match args.command {
        Commands::Listen => {
            println!("Starting listener...");
            listen(id_allocator, vm_pool, s3_service, heartbeat_store).await?;
        }

        Commands::Serve => {
            serve(id_allocator, vm_pool, s3_service, heartbeat_store).await?;
        }

        Commands::Deploy {
            url: _,
            install,
            build,
            run,
            branch,
            dist_dir,
            home_dir,
        } => {
            let deploy_req = DeployReq {
                branch: Some(branch),
                build: Some(build),
                run: Some(run),
                dist_dir,
                home_dir,
                install: Some(install),
                full_name: "test".to_string(),
                installation_id: 1,
                name: "test".to_string(),
            };

            deploy(deploy_req).await?;
        }

        Commands::Test => {
            let github = GithubApp::new();
            let installation_access_token = github.get_installation_access_token(135164979).await?;
            let commit = github
                .get_tarball_url(
                    Some("main".to_string()),
                    "scrom-2511",
                    "shipr_test_project",
                    &installation_access_token,
                )
                .await?;
            println!("{:?}", commit);
        }
    }

    Ok(())
}
