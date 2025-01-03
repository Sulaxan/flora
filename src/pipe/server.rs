use std::{
    io,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use tokio::{
    io::Interest,
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
};

use crate::{window, CONTENT};

use super::protocol::{ServerRequest, ServerResponse};

const PIPE_NAME_PREFIX: &str = r"\\.\pipe\flora";

pub fn start_server() -> Result<()> {
    let pid = std::process::id();
    let pipe_name = format!("{PIPE_NAME_PREFIX}-{pid}");
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&pipe_name)?;

    tokio::spawn(async move {
        loop {
            server.connect().await.unwrap();
            let connected_client = server;

            // create a new server to start listening for more connections
            server = ServerOptions::new().create(&pipe_name).unwrap();

            tokio::spawn(async move {
                handle_client(connected_client).await;
            });
        }
    });

    Ok(())
}

async fn handle_client(client: NamedPipeServer) -> Result<()> {
    let responses: Vec<ServerResponse> = Vec::new();
    loop {
        let ready = client
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;

        if ready.is_readable() {
            let mut data = vec![0; 1024];

            match client.try_read(&mut data) {
                Ok(n) => {
                    responses.push(handle_request(serde_json::from_slice(&data)?));
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        if ready.is_writable() {
            for response in responses.iter() {
                match client.try_write(&serde_json::to_vec(response)?) {
                    Ok(_) => (),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
        }
    }
}

fn handle_request(request: ServerRequest) -> ServerResponse {
    return match request {
        ServerRequest::GetName => ServerResponse::Name("null".to_string()),
        ServerRequest::GetContent => {
            let content = CONTENT.lock().unwrap();
            ServerResponse::Content(content.to_string())
        }
    };
}
