pub mod client;
pub mod protocol;
pub mod server;

const PIPE_NAME_PREFIX: &str = r"\\.\pipe\flora";

pub fn create_pipe_name(pid: u32) -> String {
    format!("{PIPE_NAME_PREFIX}-{pid}")
}
