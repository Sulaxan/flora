use std::io;

use anyhow::Result;
use tokio::{io::Interest, net::windows::named_pipe::ClientOptions};

use super::protocol::{ServerRequest, ServerResponse};

pub async fn send(pipe_name: &str, request: &ServerRequest) -> Result<ServerResponse> {
    let client = ClientOptions::new().open(&pipe_name)?;

    loop {
        let ready = client.ready(Interest::WRITABLE).await?;

        if ready.is_writable() {
            match client.try_write(&serde_json::to_vec(request)?) {
                Ok(_) => break,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    // *may* need to increase in the future, hopefully not...
    let mut data = vec![0; 1024];
    let read_len;
    // FIXME: timeout this in the future using tokio::select!
    loop {
        let ready = client.ready(Interest::READABLE).await?;

        if ready.is_readable() {
            match client.try_read(&mut data) {
                Ok(n) => {
                    read_len = n;
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    Ok(serde_json::from_slice(&data[0..read_len])?)
}
