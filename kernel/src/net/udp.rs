// VikingOS Kernel — User Datagram Protocol (UDP)
//
// Layer 4 transport for DNS queries and lightweight packet exchange.

#![allow(dead_code)]

use alloc::vec::Vec;

pub struct UdpPacket<'a> {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
    pub payload: &'a [u8],
}

impl<'a> UdpPacket<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let length = u16::from_be_bytes([data[4], data[5]]);
        let checksum = u16::from_be_bytes([data[6], data[7]]);

        let payload_len = (length as usize).saturating_sub(8).min(data.len() - 8);

        Some(Self {
            src_port,
            dst_port,
            length,
            checksum,
            payload: &data[8..8 + payload_len],
        })
    }

    pub fn build(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
        let length = 8 + payload.len();
        let mut packet = Vec::with_capacity(length);
        packet.extend_from_slice(&src_port.to_be_bytes());
        packet.extend_from_slice(&dst_port.to_be_bytes());
        packet.extend_from_slice(&(length as u16).to_be_bytes());
        packet.extend_from_slice(&[0, 0]); // Checksum optional in IPv4 (0 = none)
        packet.extend_from_slice(payload);
        packet
    }
}
