// VikingOS Kernel — Address Resolution Protocol (ARP)
//
// Translates IPv4 addresses to Ethernet MAC addresses and manages ARP cache.

#![allow(dead_code)]

use alloc::vec::Vec;
use super::ethernet::MacAddress;

pub const ARP_HTYPE_ETHERNET: u16 = 1;
pub const ARP_PTYPE_IPV4: u16 = 0x0800;
pub const ARP_OP_REQUEST: u16 = 1;
pub const ARP_OP_REPLY: u16 = 2;

#[derive(Clone, Copy, Debug)]
pub struct ArpPacket {
    pub htype: u16,
    pub ptype: u16,
    pub hlen: u8,
    pub plen: u8,
    pub opcode: u16,
    pub sender_mac: MacAddress,
    pub sender_ip: [u8; 4],
    pub target_mac: MacAddress,
    pub target_ip: [u8; 4],
}

impl ArpPacket {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }

        let htype = u16::from_be_bytes([data[0], data[1]]);
        let ptype = u16::from_be_bytes([data[2], data[3]]);
        let hlen = data[4];
        let plen = data[5];
        let opcode = u16::from_be_bytes([data[6], data[7]]);

        let mut sender_mac = [0u8; 6];
        sender_mac.copy_from_slice(&data[8..14]);
        let mut sender_ip = [0u8; 4];
        sender_ip.copy_from_slice(&data[14..18]);

        let mut target_mac = [0u8; 6];
        target_mac.copy_from_slice(&data[18..24]);
        let mut target_ip = [0u8; 4];
        target_ip.copy_from_slice(&data[24..28]);

        Some(Self {
            htype,
            ptype,
            hlen,
            plen,
            opcode,
            sender_mac: MacAddress(sender_mac),
            sender_ip,
            target_mac: MacAddress(target_mac),
            target_ip,
        })
    }

    pub fn build(
        opcode: u16,
        sender_mac: MacAddress,
        sender_ip: [u8; 4],
        target_mac: MacAddress,
        target_ip: [u8; 4],
    ) -> Vec<u8> {
        let mut packet = Vec::with_capacity(28);
        packet.extend_from_slice(&ARP_HTYPE_ETHERNET.to_be_bytes());
        packet.extend_from_slice(&ARP_PTYPE_IPV4.to_be_bytes());
        packet.push(6); // Hardware address length (MAC)
        packet.push(4); // Protocol address length (IPv4)
        packet.extend_from_slice(&opcode.to_be_bytes());
        packet.extend_from_slice(&sender_mac.0);
        packet.extend_from_slice(&sender_ip);
        packet.extend_from_slice(&target_mac.0);
        packet.extend_from_slice(&target_ip);
        packet
    }
}

/// Simple static ARP cache entry
#[derive(Clone, Copy, Debug)]
pub struct ArpEntry {
    pub ip: [u8; 4],
    pub mac: MacAddress,
}
