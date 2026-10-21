// VikingOS Kernel — Internet Protocol v4 (Layer 3)
//
// IPv4 packet parsing, checksum computation, and packet construction.

#![allow(dead_code)]

use alloc::vec::Vec;

pub const IP_PROTO_ICMP: u8 = 1;
pub const IP_PROTO_TCP: u8 = 6;
pub const IP_PROTO_UDP: u8 = 17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    pub const LOOPBACK: Self = Self([127, 0, 0, 1]);
    pub const BROADCAST: Self = Self([255, 255, 255, 255]);
    pub const ZERO: Self = Self([0, 0, 0, 0]);

    pub fn to_string(&self) -> alloc::string::String {
        alloc::format!("{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

pub struct Ipv4Packet<'a> {
    pub version: u8,
    pub ihl: u8,
    pub tos: u8,
    pub total_length: u16,
    pub id: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_ip: Ipv4Address,
    pub dst_ip: Ipv4Address,
    pub payload: &'a [u8],
}

impl<'a> Ipv4Packet<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let ver_ihl = data[0];
        let version = ver_ihl >> 4;
        let ihl = ver_ihl & 0x0F;

        if version != 4 || ihl < 5 {
            return None;
        }

        let header_len = (ihl as usize) * 4;
        if data.len() < header_len {
            return None;
        }

        let tos = data[1];
        let total_length = u16::from_be_bytes([data[2], data[3]]);
        let id = u16::from_be_bytes([data[4], data[5]]);
        let ttl = data[8];
        let protocol = data[9];
        let checksum = u16::from_be_bytes([data[10], data[11]]);

        let mut src = [0u8; 4];
        let mut dst = [0u8; 4];
        src.copy_from_slice(&data[12..16]);
        dst.copy_from_slice(&data[16..20]);

        let payload_len = (total_length as usize).saturating_sub(header_len).min(data.len() - header_len);
        let payload = &data[header_len..header_len + payload_len];

        Some(Self {
            version,
            ihl,
            tos,
            total_length,
            id,
            ttl,
            protocol,
            checksum,
            src_ip: Ipv4Address(src),
            dst_ip: Ipv4Address(dst),
            payload,
        })
    }

    pub fn calculate_checksum(header: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        let mut i = 0;
        while i < header.len() {
            let word = if i + 1 < header.len() {
                u16::from_be_bytes([header[i], header[i + 1]])
            } else {
                u16::from_be_bytes([header[i], 0])
            };
            sum += word as u32;
            i += 2;
        }

        while (sum >> 16) > 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !(sum as u16)
    }

    pub fn build(
        src_ip: Ipv4Address,
        dst_ip: Ipv4Address,
        protocol: u8,
        id: u16,
        ttl: u8,
        payload: &[u8],
    ) -> Vec<u8> {
        let total_len = 20 + payload.len();
        let mut header = [0u8; 20];
        header[0] = 0x45; // Version 4, IHL 5 (20 bytes)
        header[1] = 0x00; // Type of service
        header[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
        header[4..6].copy_from_slice(&id.to_be_bytes());
        header[6..8].copy_from_slice(&0x4000u16.to_be_bytes()); // Don't fragment flag
        header[8] = ttl;
        header[9] = protocol;
        header[10..12].copy_from_slice(&[0, 0]); // Zero checksum for calculation
        header[12..16].copy_from_slice(&src_ip.0);
        header[16..20].copy_from_slice(&dst_ip.0);

        let csum = Self::calculate_checksum(&header);
        header[10..12].copy_from_slice(&csum.to_be_bytes());

        let mut packet = Vec::with_capacity(total_len);
        packet.extend_from_slice(&header);
        packet.extend_from_slice(payload);
        packet
    }
}
