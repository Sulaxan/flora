use std::io;

use anyhow::Result;
use tokio::{
    io::Interest,
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
};
use tracing::{info, trace};
use windows::Win32::Foundation::ERROR_NO_DATA;

use crate::{
    execute,
    windows_api::{self},
    CONTENT, NAME,
};

use super::{
    create_pipe_name,
    protocol::{ServerRequest, ServerResponse},
};

#[tracing::instrument]
pub async fn start_server() -> Result<()> {
    info!("starting named pipe server");
    let pid = std::process::id();
    let pipe_name = create_pipe_name(pid);
    info!(pid, pipe_name);
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&pipe_name)?;

    loop {
        server.connect().await.unwrap();
        let connected_client = server;

        info!("client connected to named pipe");

        // create a new server to start listening for more connections
        server = ServerOptions::new().create(&pipe_name).unwrap();

        tokio::spawn(async move {
            handle_client(connected_client).await.unwrap();
        });
    }
}

async fn handle_client(client: NamedPipeServer) -> Result<()> {
    let mut responses: Vec<ServerResponse> = Vec::new();
    loop {
        let ready = client
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;

        if ready.is_readable() {
            let mut data = vec![0; 1024];

            match client.try_read(&mut data) {
                Ok(0) => break,
                Ok(n) => {
                    let request = serde_json::from_slice(&data[0..n])?;
                    trace!(
                        bytes = n,
                        data = &data[0..n],
                        "received request from client"
                    );
                    responses.push(handle_request(request));
                }
                Err(e) if e.raw_os_error() == Some(ERROR_NO_DATA.0 as i32) => break,
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
                    Err(e) if e.raw_os_error() == Some(ERROR_NO_DATA.0 as i32) => break,
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

    info!("client disconnected from named pipe");
    Ok(())
}

#[tracing::instrument(level = "trace")]
fn handle_request(request: ServerRequest) -> ServerResponse {
    return match request {
        ServerRequest::GetName => {
            let name = NAME.lock().unwrap();
            ServerResponse::Name(name.to_string())
        }
        ServerRequest::GetContent => {
            let content = CONTENT.lock().unwrap();
            ServerResponse::Content(content.to_string())
        }
        ServerRequest::ShowWindow => {
            return execute(|webview| {
                windows_api::show_window(*webview.hwnd);
            })
            .map_or_else(
                |e| ServerResponse::Err(format!("{e:?}")),
                |_| ServerResponse::Ok,
            );
        }
        ServerRequest::HideWindow => {
            return execute(|webview| {
                windows_api::hide_window(*webview.hwnd);
            })
            .map_or_else(
                |e| ServerResponse::Err(format!("{e:?}")),
                |_| ServerResponse::Ok,
            );
        }
    };
}
