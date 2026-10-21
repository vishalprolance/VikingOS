// VikingOS Kernel — Ethernet Protocol (Layer 2)
//
// Ethernet II frame encoding and decoding for bare-metal network packets.

#![allow(dead_code)]

use alloc::vec::Vec;

pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV6: u16 = 0x86DD;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: Self = Self([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    pub const ZERO: Self = Self([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    pub fn to_string(&self) -> alloc::string::String {
        alloc::format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// Ethernet II Frame Header (14 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct EthernetHeader {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ethertype: u16,
}

pub struct EthernetFrame<'a> {
    pub dst_mac: MacAddress,
    pub src_mac: MacAddress,
    pub ethertype: u16,
    pub payload: &'a [u8],
}

impl<'a> EthernetFrame<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 14 {
            return None;
        }

        let mut dst = [0u8; 6];
        let mut src = [0u8; 6];
        dst.copy_from_slice(&data[0..6]);
        src.copy_from_slice(&data[6..12]);
        let ethertype = u16::from_be_bytes([data[12], data[13]]);

        Some(Self {
            dst_mac: MacAddress(dst),
            src_mac: MacAddress(src),
            ethertype,
            payload: &data[14..],
        })
    }

    pub fn build(dst: MacAddress, src: MacAddress, ethertype: u16, payload: &[u8]) -> Vec<u8> {
        let mut packet = Vec::with_capacity(14 + payload.len());
        packet.extend_from_slice(&dst.0);
        packet.extend_from_slice(&src.0);
        packet.extend_from_slice(&ethertype.to_be_bytes());
        packet.extend_from_slice(payload);
        packet
    }
}
