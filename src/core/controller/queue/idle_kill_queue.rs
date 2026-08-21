use futures::StreamExt;
use lapin::{
    Channel, Queue,
    options::{BasicAckOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, LongString, ShortString},
};
use serde::{Deserialize, Serialize};

use crate::{
    app_errors::AppError, core::controller::queue::lapin::Lapin,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdleKillReq {
    pub project_id: String,
    pub numeric_id: i32,
}

#[derive(Clone)]
pub struct IdleKillQueue {
    channel: Channel,
    queue: Queue,
}

impl IdleKillQueue {
    pub async fn new(lapin_conn: &Lapin) -> Result<Self, AppError> {
        let connection = lapin_conn.get_connection().await;

        let channel = connection
            .create_channel()
            .await
            .map_err(|e| AppError::ChannelError(e.to_string()))?;

        let mut queue_args = FieldTable::default();
        queue_args.insert(
            ShortString::from("x-queue-type"),
            AMQPValue::LongString(LongString::from("quorum")),
        );

        let queue = channel
            .queue_declare(
                ShortString::from("idle_kill_queue"),
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                queue_args,
            )
            .await
            .map_err(|e| AppError::QueueError(e.to_string()))?;

        Ok(Self { channel, queue })
    }

    pub async fn add_to_queue(&self, idle_req: &IdleKillReq) -> Result<(), AppError> {
        self.channel
            .basic_publish(
                ShortString::from(""),
                ShortString::from("idle_kill_queue"),
                Default::default(),
                serde_json::to_string(idle_req).unwrap().as_bytes(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        println!("Published idle kill job for project: {}", idle_req.project_id);

        Ok(())
    }

    pub async fn pop_from_queue(&self) -> Result<IdleKillReq, AppError> {
        let mut consumer = self
            .channel
            .basic_consume(
                ShortString::from("idle_kill_queue"),
                ShortString::from("idle_kill_queue"),
                Default::default(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.map_err(|e| AppError::LapinError(e.to_string()))?;

            let data = serde_json::from_slice::<IdleKillReq>(&delivery.data)
                .map_err(|e| AppError::LapinError(e.to_string()))?;

            delivery
                .ack(BasicAckOptions::default())
                .await
                .map_err(|e| AppError::LapinError(e.to_string()))?;

            return Ok(data);
        }

        Err(AppError::QueueError("No message received".to_string()))
    }
}
