use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

pub const PORT: u16 = 56863;

#[derive(FromPrimitive, ToPrimitive, Debug)]
pub enum MessageType {
    /// Simple ping to verify that server exists and is accessible
    Ping,
    /// Sent to register the client as actively taking the test
    Register,
    /// Transmits large blocks (many MBs) of data
    Transmit,
}

pub fn parse_message_type(value: u8) -> std::io::Result<MessageType> {
    let Some(message_type) = MessageType::from_u8(value) else {
        return std::io::Result::Err(std::io::Error::other("Unknown message type"));
    };
    return Ok(message_type);
}
