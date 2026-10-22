#![allow(dead_code)]

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use std::collections::HashMap;

pub const PORT: u16 = 56863;
pub const MAX_NAME_LENGTH: usize = 32;

/// Take a screenshot every n seconds
pub const SCREENSHOT_INTERVAL: u8 = 2;

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Eq)]
pub enum ClientMessageType {
    /// Simple ping to verify that server exists and is accessible
    Ping,
    /// Sent to register the client as actively taking the test
    Register,
    /// Transmits large blocks (many MBs) of data
    SubmitExam,
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Eq)]
pub enum ServerMessageType {
    /// Reply to client's ping
    Pong,
    /// Reply to client's register
    RegisterStatus,
}

pub fn parse_message_type<T: FromPrimitive>(value: u8) -> std::io::Result<T> {
    let Some(message_type) = T::from_u8(value) else {
        return std::io::Result::Err(std::io::Error::other(format!(
            "Unknown message type: {value}"
        )));
    };
    Ok(message_type)
}

pub struct WindowOpenTime {
    pub opened: i64,
    pub closed: i64,
}

pub type OpenedWindows = HashMap<String, Vec<WindowOpenTime>>;
pub fn serialize_opened_windows(v: OpenedWindows) -> String {
    // serialized data follows the format of:
    //      KEY1:OPENTIME1-CLOSETIME1,OPENTIME2-CLOSETIME2
    //      KEY2:OPENTIME1-CLOSETIME1
    // etc

    let mut buf = String::new();
    for (k, v) in v.into_iter() {
        buf += &(k + ":");
        for time in v {
            buf += &format!("{}-{},", time.opened, time.closed);
        }
        buf.pop(); // remove last comma
        buf += "\n";
    }
    buf
}
