use hickory_proto::{
    op::{Message, MessageType, OpCode},
    rr::{RData, Record, rdata::A},
};

use crate::{
    app_errors::AppError,
    core::{app_types::JobType, controller::vm::vm_pool::VmPool},
};

#[derive(Clone)]
pub struct ShiprDNS {
    vm_pool: VmPool,
}

impl ShiprDNS {
    pub fn new(vm_pool: VmPool) -> Self {
        Self { vm_pool }
    }

    pub async fn start(&self) -> Result<(), AppError> {
        let socket = tokio::net::UdpSocket::bind("192.168.29.105:53").await?;

        let mut buf = [0u8; 512];

        loop {
            let (size, peer) = socket.recv_from(&mut buf).await?;

            // println!("Received {} bytes from {}", size, peer);

            let response = self.handle_packet(&buf[..size]).await?;

            socket.send_to(&response, peer).await?;
        }
    }

    async fn handle_packet(&self, packet: &[u8]) -> Result<Vec<u8>, AppError> {
        let request = Message::from_vec(packet)?;

        let query = request
            .queries
            .first()
            .ok_or(AppError::BadRequest("No query found".to_string()))?;

        let domain = query.name.to_string();

        // println!("Received request for domain {}", domain);

        if domain.contains("shipr.com.") {
            // println!("Received request for shipr.com domain");
            let project_id = domain.replace(".shipr.com.", "");

            let _vm = self
                .vm_pool
                .get_from_pool(&project_id, &JobType::Run)
                .await?;

            let mut response =
                Message::new(request.metadata.id, MessageType::Response, OpCode::Query);

            response.add_query(query.clone());

            // match vm {
            //     Some(vm) => {
            //         let vm_ip_addr = format!("172.16.0.{}", vm * 4 + 2).parse().unwrap();

            //         println!("VM IP address: {}", vm_ip_addr);

            //         response.add_answer(Record::from_rdata(
            //             query.name.clone(),
            //             30,
            //             RData::A(A(vm_ip_addr)),
            //         ));
            //     }
            //     None => {
            //         response.metadata.response_code = ResponseCode::NXDomain;
            //     }
            // }

            response.add_answer(Record::from_rdata(
                query.name.clone(),
                30,
                RData::A(A("127.0.0.1".parse().unwrap())),
            ));

            return Ok(response.to_vec()?);
        }

        println!("Forwarding non-shipr domain");
        self.forward_query(packet).await
    }

    async fn forward_query(&self, packet: &[u8]) -> Result<Vec<u8>, AppError> {
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;

        let upstream = "1.1.1.1:53";

        socket.send_to(packet, upstream).await?;

        let mut buf = [0u8; 512];

        let (size, _) = socket.recv_from(&mut buf).await?;

        Ok(buf[..size].to_vec())
    }
}
