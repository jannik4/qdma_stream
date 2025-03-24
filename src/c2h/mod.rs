use crate::{
    util::{mem_aligned, mem_aligned_free},
    ALIGN, CTRL_SIZE, PACKET_SIZE,
};
use anyhow::Result;
use std::{
    io::{self, Read, Write},
    ptr::NonNull,
};

unsafe impl<F> Send for CardToHostStream<F> {}
unsafe impl<F> Sync for CardToHostStream<F> {}

pub struct CardToHostStream<F> {
    file: F,
    ptr: NonNull<u8>,
    ptr_prev: NonNull<u8>,
    ptr_ctrl: NonNull<u8>,

    protocol_state: ProtocolState,
}

impl<F> CardToHostStream<F> {
    pub fn new(file: F) -> Result<Self> {
        let ptr = mem_aligned(PACKET_SIZE, ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), PACKET_SIZE) };
        slice.copy_from_slice(&[0; PACKET_SIZE]);

        let ptr_prev = mem_aligned(PACKET_SIZE, ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr_prev.as_ptr(), PACKET_SIZE) };
        slice.copy_from_slice(&[0; PACKET_SIZE]);

        let ptr_ctrl = mem_aligned(CTRL_SIZE, ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr_ctrl.as_ptr(), CTRL_SIZE) };
        slice.copy_from_slice(&[0; CTRL_SIZE]);

        Ok(Self {
            file,
            ptr,
            ptr_prev,
            ptr_ctrl,

            protocol_state: ProtocolState::NotSet,
        })
    }
}

impl<F> CardToHostStream<F>
where
    F: Read,
{
    pub fn next_raw_packet_with_len(&mut self, len: usize) -> io::Result<&[u8]> {
        let len = usize::min(len, PACKET_SIZE);
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), len) };

        self.file.read_exact(slice)?;
        Ok(slice)
    }

    pub fn next_raw_packet(&mut self) -> io::Result<&[u8]> {
        self.next_raw_packet_with_len(PACKET_SIZE)
    }

    pub fn read_complete_stream(&mut self, mut buf: impl Write) -> io::Result<usize> {
        let mut bytes = 0;
        loop {
            let (is_last, packet) = self.next_stream_packet()?;
            buf.write_all(packet)?;
            bytes += packet.len();
            if is_last {
                break Ok(bytes);
            }
        }
    }

    /// Returns `(is_last, data)`
    pub fn next_stream_packet(&mut self) -> io::Result<(bool, &[u8])> {
        // Read previous packet
        let slice_prev =
            unsafe { std::slice::from_raw_parts_mut(self.ptr_prev.as_ptr(), PACKET_SIZE) };
        match self.protocol_state {
            ProtocolState::NotSet => {
                self.protocol_state = ProtocolState::Data;
                match self.next_beat_protocol(slice_prev)? {
                    BeatMeta::ThisIsData => (),
                    BeatMeta::ThisIsLast(len) => {
                        self.protocol_state = ProtocolState::NotSet;
                        return Ok((true, &slice_prev[..len]));
                    }
                    BeatMeta::PrevIsLast(_) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "protocol error"));
                    }
                }
            }
            ProtocolState::Data => (),
            ProtocolState::Last(len) => {
                self.protocol_state = ProtocolState::NotSet;
                let slice =
                    unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), PACKET_SIZE) };
                return Ok((true, &slice[..len]));
            }
        }

        // Read current packet
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), PACKET_SIZE) };
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

    fn next_beat_protocol(&mut self, slice: &mut [u8]) -> io::Result<BeatMeta> {
        self.file.read_exact(slice)?;
        if slice.starts_with(&CTRL_SEQ) {
            self.read_ctrl()
        } else {
            Ok(BeatMeta::ThisIsData)
        }
    }

    fn read_ctrl(&mut self) -> io::Result<BeatMeta> {
        let slice_ctrl =
            unsafe { std::slice::from_raw_parts_mut(self.ptr_ctrl.as_ptr(), CTRL_SIZE) };
        self.file.read_exact(slice_ctrl)?;
        let ctrl = u32::from_le_bytes([slice_ctrl[0], slice_ctrl[1], slice_ctrl[2], slice_ctrl[3]]);

        Ok(if ctrl == 0 {
            BeatMeta::ThisIsData
        } else if (ctrl & (1 << 31)) == 0 {
            let len = usize::min(PACKET_SIZE, ctrl as usize);
            BeatMeta::ThisIsLast(len)
        } else {
            let len = usize::min(PACKET_SIZE, (ctrl & !(1 << 31)) as usize);
            BeatMeta::PrevIsLast(len)
        })
    }
}

impl<F> Drop for CardToHostStream<F> {
    fn drop(&mut self) {
        unsafe {
            mem_aligned_free(self.ptr.as_ptr(), PACKET_SIZE, ALIGN);
            mem_aligned_free(self.ptr_ctrl.as_ptr(), CTRL_SIZE, ALIGN);
        }
    }
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

const CTRL_SEQ: [u8; 4] = [0x5C, 0xF1, 0x37, 0x4A];
