use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

pub const PORT: u16 = 56863;

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Eq)]
pub enum ClientMessageType {
    /// Simple ping to verify that server exists and is accessible
    Ping,
    /// Sent to register the client as actively taking the test
    Register,
    /// Transmits large blocks (many MBs) of data
    Transmit,
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Eq)]
pub enum ServerMessageType {
    /// Reply to client's ping
    Pong,
    /// Reply to client's register
    RegisterStatus,
    /// Reply to client's transmit
    TransmitSuccess,
}

pub fn parse_message_type<T: FromPrimitive>(value: u8) -> std::io::Result<T> {
    let Some(message_type) = T::from_u8(value) else {
        return std::io::Result::Err(std::io::Error::other(format!(
            "Unknown message type: {value}"
        )));
    };
    return Ok(message_type);
}
