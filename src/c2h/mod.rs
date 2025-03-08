use crate::util::{mem_aligned, mem_aligned_free};
use anyhow::{bail, Result};
use std::{fs, io::Read, ptr::NonNull};

pub struct CardToHostStream {
    file: fs::File,
    ptr: NonNull<u8>,
    ptr_prev: NonNull<u8>,
    ptr_ctrl: NonNull<u8>,

    protocol_state: ProtocolState,
}

impl CardToHostStream {
    pub const PACKET_SIZE: usize = 4096;
    const ALIGN: usize = 4096;
    const CTRL_SIZE: usize = 4;

    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = fs::OpenOptions::new().read(true).open(path.as_ref())?;

        let ptr = mem_aligned(Self::PACKET_SIZE, Self::ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), Self::PACKET_SIZE) };
        slice.copy_from_slice(&[0; Self::PACKET_SIZE]);

        let ptr_prev = mem_aligned(Self::PACKET_SIZE, Self::ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr_prev.as_ptr(), Self::PACKET_SIZE) };
        slice.copy_from_slice(&[0; Self::PACKET_SIZE]);

        let ptr_ctrl = mem_aligned(Self::CTRL_SIZE, Self::ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr_ctrl.as_ptr(), Self::CTRL_SIZE) };
        slice.copy_from_slice(&[0; Self::CTRL_SIZE]);

        Ok(Self {
            file,
            ptr,
            ptr_prev,
            ptr_ctrl,

            protocol_state: ProtocolState::NotSet,
        })
    }

    pub fn read_next(&mut self, len: usize) -> Result<&[u8]> {
        let len = usize::min(len, Self::PACKET_SIZE);
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), len) };

        self.file.read_exact(slice)?;
        Ok(slice)
    }

    pub fn next_packet(&mut self) -> Result<&[u8]> {
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), Self::PACKET_SIZE) };

        self.file.read_exact(slice)?;
        Ok(slice)
    }

    pub fn next_packet_or_ctrl_seq(&mut self) -> Result<PacketOrCtrlSeq<'_>> {
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), Self::PACKET_SIZE) };

        self.file.read_exact(slice)?;
        if slice.starts_with(&CTRL_SEQ) {
            self.file.read_exact(slice)?;
            if slice.starts_with(&CTRL_SEQ) {
                Ok(PacketOrCtrlSeq::Packet(slice))
            } else {
                Ok(PacketOrCtrlSeq::CtrlSeq(slice))
            }
        } else {
            Ok(PacketOrCtrlSeq::Packet(slice))
        }
    }

    pub fn next_packet_or_eof(&mut self) -> Result<Option<&[u8]>> {
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), Self::PACKET_SIZE) };
        let slice_ctrl =
            unsafe { std::slice::from_raw_parts_mut(self.ptr_ctrl.as_ptr(), Self::CTRL_SIZE) };

        self.file.read_exact(slice)?;
        if slice.starts_with(&CTRL_SEQ) {
            self.file.read_exact(slice_ctrl)?;
            if slice_ctrl[0] == 0 {
                Ok(Some(slice))
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(slice))
        }
    }

    /// Returns `(is_last, data)`
    pub fn next_packet_protocol(&mut self) -> Result<(bool, &[u8])> {
        // Read previous packet
        let slice_prev =
            unsafe { std::slice::from_raw_parts_mut(self.ptr_prev.as_ptr(), Self::PACKET_SIZE) };
        match self.protocol_state {
            ProtocolState::NotSet => {
                self.protocol_state = ProtocolState::Data;
                match self.next_beat_protocol(slice_prev)? {
                    BeatMeta::ThisIsData => (),
                    BeatMeta::ThisIsLast(len) => {
                        self.protocol_state = ProtocolState::NotSet;
                        return Ok((true, &slice_prev[..len]));
                    }
                    BeatMeta::PrevIsLast(_) => bail!("protocol error"),
                }
            }
            ProtocolState::Data => (),
            ProtocolState::Last(len) => {
                self.protocol_state = ProtocolState::NotSet;
                let slice =
                    unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), Self::PACKET_SIZE) };
                return Ok((true, &slice[..len]));
            }
        }

        // Read current packet
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), Self::PACKET_SIZE) };
        match self.next_beat_protocol(slice)? {
            BeatMeta::ThisIsData => {
                // Swap pointers
                std::mem::swap(&mut self.ptr, &mut self.ptr_prev);
            }
            BeatMeta::ThisIsLast(len) => self.protocol_state = ProtocolState::Last(len),
            BeatMeta::PrevIsLast(len) => {
                self.protocol_state = ProtocolState::NotSet;
                return Ok((true, &slice_prev[..len]));
            }
        }

        Ok((false, slice_prev))
    }

    fn next_beat_protocol(&mut self, slice: &mut [u8]) -> Result<BeatMeta> {
        self.file.read_exact(slice)?;
        // dbg!((&slice[0..4], &slice[slice.len() - 4..]));
        // dbg!(&slice);
        for (i, byte) in slice.iter().enumerate() {
            if i != 0 {
                print!(", ");
            }
            print!("{}", byte);
        }
        println!();
        if slice.starts_with(&CTRL_SEQ) {
            self.read_ctrl()
        } else {
            Ok(BeatMeta::ThisIsData)
        }
    }

    fn read_ctrl(&mut self) -> Result<BeatMeta> {
        let slice_ctrl =
            unsafe { std::slice::from_raw_parts_mut(self.ptr_ctrl.as_ptr(), Self::CTRL_SIZE) };
        self.file.read_exact(slice_ctrl)?;
        let ctrl = u32::from_le_bytes([slice_ctrl[0], slice_ctrl[1], slice_ctrl[2], slice_ctrl[3]]);

        Ok(if ctrl == 0 {
            BeatMeta::ThisIsData
        } else if (ctrl & (1 << 31)) == 0 {
            let len = usize::min(Self::PACKET_SIZE, ctrl as usize);
            BeatMeta::ThisIsLast(len)
        } else {
            let len = usize::min(Self::PACKET_SIZE, (ctrl & !(1 << 31)) as usize);
            BeatMeta::PrevIsLast(len)
        })
    }
}

impl Drop for CardToHostStream {
    fn drop(&mut self) {
        unsafe {
            mem_aligned_free(self.ptr.as_ptr(), Self::PACKET_SIZE, Self::ALIGN);
            mem_aligned_free(self.ptr_ctrl.as_ptr(), Self::CTRL_SIZE, Self::ALIGN);
        }
    }
}

pub enum PacketOrCtrlSeq<'a> {
    Packet(&'a [u8]),
    CtrlSeq(&'a [u8]),
}

#[derive(Debug, Clone, Copy)]
enum ProtocolState {
    NotSet,
    Data,
    Last(usize),
}

#[derive(Debug, Clone, Copy)]
enum BeatMeta {
    ThisIsData,
    ThisIsLast(usize),
    PrevIsLast(usize),
}

const CTRL_SEQ: [u8; 4] = [0x4A, 0x37, 0xF1, 0x5C];
