#[derive(Debug, Clone, PartialEq)]
pub enum ClientState {
    NotIdentified,
    Identified,
    Consumer(String),
    Quiet(String),
    Terminating(String),
    End,
}
