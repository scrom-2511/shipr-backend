use std::{fs, io::Write};

use actix_web::{
    HttpResponse,
    web::{self, Bytes},
};
use serde_json::Value;

use crate::{app_errors::AppError, core::app_types::LogsStore};

pub async fn stream_logs_handler(
    path: web::Path<String>,
    logs_store: LogsStore,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();

    println!("Project ID: {}", project_id);

    let sender = logs_store.lock().await.get(&project_id).cloned();

    let sender = match sender {
        Some(sender) => sender,
        None => {
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    let mut rx = sender.subscribe();

    let stream = async_stream::stream! {

        let file_path = "/home/scrom/code/shipr/shipr-backend/logs";

        let old_logs = fs::read_to_string(format!("{}/{}.txt", file_path, project_id)).unwrap();

        for line in old_logs.lines() {
            yield Ok::<Bytes, actix_web::Error>(
                Bytes::from(
                    line.to_owned()
                )
            );
        }

        loop {
            match rx.recv().await {
                Ok(msg) => {
                    yield Ok::<Bytes, actix_web::Error>(
                        Bytes::from(
                            msg
                        )
                    );
                }

                Err(_) => {
                    break;
                }
            }
        }
    };

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .streaming(stream))
}

pub async fn logs_handler(
    logs_store: LogsStore,
    data: web::Bytes,
) -> Result<HttpResponse, AppError> {
    let data = serde_json::from_slice::<Value>(&data).unwrap();

    let project_id = data["project_id"].as_str().unwrap();

    let log = data["log"].as_str().unwrap();

    let file_path = "/home/scrom/code/shipr/shipr-backend/logs";

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/{}.txt", file_path, project_id))
        .unwrap();

    writeln!(file, "{}", log).unwrap();

    let tx = logs_store.lock().await.get(project_id).cloned();

    if let Some(tx) = tx {
        tx.send(log.to_string())?;
    }

    Ok(HttpResponse::Ok().finish())
}
