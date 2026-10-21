// VikingOS Kernel — Transmission Control Protocol (TCP)
//
// Layer 4 stateful connection management for HTTP client and web socket operations.

#![allow(dead_code)]

use alloc::vec::Vec;

pub const TCP_FLAG_FIN: u8 = 0x01;
pub const TCP_FLAG_SYN: u8 = 0x02;
pub const TCP_FLAG_RST: u8 = 0x04;
pub const TCP_FLAG_PSH: u8 = 0x08;
pub const TCP_FLAG_ACK: u8 = 0x10;
pub const TCP_FLAG_URG: u8 = 0x20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    SynSent,
    Established,
    FinWait,
    TimeWait,
}

pub struct TcpSegment<'a> {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset: u8,
    pub flags: u8,
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
    pub payload: &'a [u8],
}

impl<'a> TcpSegment<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let seq_num = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ack_num = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let data_offset = (data[12] >> 4) * 4;
        let flags = data[13];
        let window_size = u16::from_be_bytes([data[14], data[15]]);
        let checksum = u16::from_be_bytes([data[16], data[17]]);
        let urgent_ptr = u16::from_be_bytes([data[18], data[19]]);

        if data.len() < data_offset as usize {
            return None;
        }

        Some(Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset,
            flags,
            window_size,
            checksum,
            urgent_ptr,
            payload: &data[data_offset as usize..],
        })
    }

    pub fn build(
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window_size: u16,
        payload: &[u8],
    ) -> Vec<u8> {
        let total_len = 20 + payload.len();
        let mut segment = Vec::with_capacity(total_len);
        segment.extend_from_slice(&src_port.to_be_bytes());
        segment.extend_from_slice(&dst_port.to_be_bytes());
        segment.extend_from_slice(&seq_num.to_be_bytes());
        segment.extend_from_slice(&ack_num.to_be_bytes());
        segment.push((5 << 4) as u8); // 5 32-bit words (20 bytes header)
        segment.push(flags);
        segment.extend_from_slice(&window_size.to_be_bytes());
        segment.extend_from_slice(&[0, 0]); // Checksum placeholder
        segment.extend_from_slice(&[0, 0]); // Urgent pointer
        segment.extend_from_slice(payload);
        segment
    }
}
