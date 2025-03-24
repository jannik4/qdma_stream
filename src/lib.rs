mod c2h;
mod h2c;
mod util;

pub mod ctl;

pub use self::{c2h::CardToHostStream, h2c::HostToCardStream};

pub const PACKET_SIZE: usize = 4096;
pub const ALIGN: usize = 4096;
pub const CTRL_SIZE: usize = 4;
