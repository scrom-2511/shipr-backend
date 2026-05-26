use std::fs;

use shipr::{
    core::app_types::{DeployDetails, JobType, RedeployDetails, RunDetails},
    core::worker::executer::job_executer::JobExecuter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: worker <job.json> <jobtype>");
        std::process::exit(1);
    }

    let json_path = &args[1];
    let job_type = &args[2];

    let content = fs::read_to_string(json_path).unwrap();

    match job_type.as_str() {
        "deploy" => {
            println!("Deploy job received");
            let deploy_details = serde_json::from_str::<DeployDetails>(&content).unwrap();

            let worker = JobExecuter::new();

            worker
                .execute(&deploy_details, JobType::Deploy, None)
                .await?;
        }
        "run" => {
            let run_details = serde_json::from_str::<RunDetails>(&content).unwrap();

            let worker = JobExecuter::new();

            worker.run(&run_details).await?;
        }
        "redeploy" => {
            let redeploy_details = serde_json::from_str::<RedeployDetails>(&content).unwrap();

            let worker = JobExecuter::new();

            worker
                .redeploy(&redeploy_details, JobType::Redeploy)
                .await?;
        }
        _ => {
            eprintln!("Invalid job type");
            std::process::exit(1);
        }
    }

    Ok(())
}
