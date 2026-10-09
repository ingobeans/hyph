use num_derive::{FromPrimitive, ToPrimitive};

pub const PORT: u16 = 56863;

#[derive(FromPrimitive, ToPrimitive)]
pub enum MessageTypes {
    Ping,
    Register,
}
