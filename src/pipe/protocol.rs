//! Represents the protocol used by a client to talk to flora processes.
//!
//! Note that a number of assumptions are made with the current protocol:
//! - Communication is bi-directional, but requests are one way. That is to say, only the client
//!   will request something from the server, and the server will respond back.
//! - The order of responses is based on the order of the requests received.

use serde::{Deserialize, Serialize};

/// Represents accepted actions to the server by the client.
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerRequest {
    /// Get the name of the widget.
    GetName,
    /// Get the content of the widget.
    GetContent,
}

/// Represents a server response
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerResponse {
    /// The name of the widget.
    Name(String),
    /// The content of the widget
    Content(String),
}
