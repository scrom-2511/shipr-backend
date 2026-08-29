use futures::StreamExt;
use lapin::{
    Channel, Queue,
    options::{BasicAckOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, LongString, ShortString},
};

use crate::{
    app_errors::AppError, core::app_types::DeployReq, core::controller::queue::lapin::Lapin,
};

pub struct DeployQueue {
    channel: Channel,
    queue: Queue,
}

impl DeployQueue {
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
                ShortString::from("deploy_queue"),
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

    pub async fn add_to_queue(&self, deploy_details: &DeployReq) -> Result<(), AppError> {
        self.channel
            .basic_publish(
                ShortString::from(""),
                ShortString::from("deploy_queue"),
                Default::default(),
                serde_json::to_string(deploy_details).unwrap().as_bytes(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        println!("Message published");

        Ok(())
    }

    pub async fn create_consumer(&self, consumer_tag: &str) -> Result<lapin::Consumer, AppError> {
        let consumer = self
            .channel
            .basic_consume(
                ShortString::from("deploy_queue"),
                ShortString::from(consumer_tag),
                Default::default(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        Ok(consumer)
    }
}
