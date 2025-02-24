use crate::util::{mem_aligned, mem_aligned_free};
use anyhow::Result;
use std::{fs, io::Read, ptr::NonNull};

pub struct CardToHostStream {
    file: fs::File,
    ptr: NonNull<u8>,
    ptr_ctrl: NonNull<u8>,
}

impl CardToHostStream {
    pub const PACKET_SIZE: usize = 4096;
    const ALIGN: usize = 4096;
    const CTRL_SIZE: usize = 32;

    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = fs::OpenOptions::new().read(true).open(path.as_ref())?;

        let ptr = mem_aligned(Self::PACKET_SIZE, Self::ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), Self::PACKET_SIZE) };
        slice.copy_from_slice(&[0; Self::PACKET_SIZE]);

        let ptr_ctrl = mem_aligned(Self::CTRL_SIZE, Self::ALIGN)?;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr_ctrl.as_ptr(), Self::CTRL_SIZE) };
        slice.copy_from_slice(&[0; Self::CTRL_SIZE]);

        Ok(Self {
            file,
            ptr,
            ptr_ctrl,
        })
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

const CTRL_SEQ: [u8; 4] = [0x4A, 0x37, 0xF1, 0x5C];
