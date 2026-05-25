use futures::StreamExt;
use lapin::{
    Channel, Queue,
    options::{BasicAckOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, LongString, ShortString},
};

use crate::{
    app_errors::AppError, core::app_types::RedeployEvent, core::controller::queue::lapin::Lapin,
};

pub struct ReDeployQueue {
    channel: Channel,
    queue: Queue,
}

impl ReDeployQueue {
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
                ShortString::from("redeploy_queue"),
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

    pub async fn publish(&self, redeploy_event: RedeployEvent) -> Result<(), AppError> {
        self.channel
            .basic_publish(
                ShortString::from(""),
                ShortString::from("redeploy_queue"),
                Default::default(),
                serde_json::to_string(&redeploy_event).unwrap().as_bytes(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        println!("Message published");

        Ok(())
    }

    pub async fn consume(&self) -> Result<RedeployEvent, AppError> {
        let mut consumer = self
            .channel
            .basic_consume(
                ShortString::from("redeploy_queue"),
                ShortString::from("redeploy_queue"),
                Default::default(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.map_err(|e| AppError::LapinError(e.to_string()))?;

            let redeploy_event = serde_json::from_slice::<RedeployEvent>(&delivery.data)
                .map_err(|e| AppError::LapinError(e.to_string()))?;

            delivery
                .ack(BasicAckOptions::default())
                .await
                .map_err(|e| AppError::LapinError(e.to_string()))?;

            return Ok(redeploy_event);
        }

        Err(AppError::QueueError("No message received".to_string()))
    }
}
