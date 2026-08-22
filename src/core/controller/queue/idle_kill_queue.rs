use futures::StreamExt;
use lapin::{
    BasicProperties, Channel, Queue,
    message::Delivery,
    options::{
        BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::{AMQPValue, FieldTable, LongString, ShortString},
};
use serde::{Deserialize, Serialize};

use crate::{app_errors::AppError, core::controller::queue::lapin::Lapin};

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

        channel
            .exchange_declare(
                ShortString::from("idle_kill_exchange"),
                lapin::ExchangeKind::Direct,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

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

        channel
            .queue_bind(
                ShortString::from("idle_kill_queue"),
                ShortString::from("idle_kill_exchange"),
                ShortString::from("execute"),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        let mut args = FieldTable::default();

        args.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString("idle_kill_exchange".into()),
        );

        args.insert(
            "x-dead-letter-routing-key".into(),
            AMQPValue::LongString("execute".into()),
        );

        args.insert(
            "x-queue-type".into(),
            AMQPValue::LongString("quorum".into()),
        );

        channel
            .queue_declare(
                ShortString::from("idle_kill_delay_queue"),
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                args,
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        channel
            .queue_bind(
                ShortString::from("idle_kill_delay_queue"),
                ShortString::from("idle_kill_exchange"),
                ShortString::from("schedule"),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        Ok(Self { channel, queue })
    }

    pub async fn add_to_queue(&self, idle_req: &IdleKillReq) -> Result<(), AppError> {
        self.channel
            .basic_publish(
                ShortString::from("idle_kill_exchange"),
                ShortString::from("schedule"),
                BasicPublishOptions::default(),
                serde_json::to_vec(&idle_req)?.as_slice(),
                BasicProperties::default().with_expiration("120000".into()),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        println!(
            "Published idle kill job for project: {} at {} ",
            idle_req.project_id,
            chrono::Utc::now().timestamp()
        );

        Ok(())
    }

    pub async fn pop_from_queue(&self) -> Result<Delivery, AppError> {
        let mut consumer = self
            .channel
            .basic_consume(
                ShortString::from("idle_kill_queue"),
                ShortString::from("idle_kill_consumer"),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        match consumer.next().await {
            Some(delivery) => {
                let delivery = delivery.map_err(|e| AppError::LapinError(e.to_string()))?;

                Ok(delivery)
            }

            None => Err(AppError::QueueError(
                "Consumer stopped without receiving a message".to_string(),
            )),
        }
    }
}
