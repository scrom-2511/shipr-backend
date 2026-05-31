use redis::{Client, aio::MultiplexedConnection};

#[derive(Clone)]
pub struct Redis {
    conn: MultiplexedConnection,
}

impl Redis {
    pub async fn new() -> Self {
        let client = Client::open("rediss://default:gQAAAAAAAYZ4AAIgcDE1YmJjNmZmN2NlZjI0OTM0YmFmNmU3MjRkZGNjMDgzOA@steady-jackal-99960.upstash.io:6379").unwrap();

        // let client = Client::open("redis://default:fTkKKJUy3K9mv10rEVRLEOzVbLiJmyPx@redis-16064.crce276.ap-south-1-3.ec2.cloud.redislabs.com:16064").unwrap();

        let conn = client.get_multiplexed_async_connection().await.unwrap();

        Self { conn }
    }

    pub fn get_conn(&self) -> MultiplexedConnection {
        self.conn.clone()
    }
}
