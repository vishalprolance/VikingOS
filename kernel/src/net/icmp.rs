// VikingOS Kernel — Internet Control Message Protocol (ICMP)
//
// Supports ICMP Echo Requests and Replies (ping diagnostics).

#![allow(dead_code)]

use alloc::vec::Vec;
use super::ipv4::Ipv4Packet;

pub const ICMP_TYPE_ECHO_REPLY: u8 = 0;
pub const ICMP_TYPE_ECHO_REQUEST: u8 = 8;

#[derive(Clone, Debug)]
pub struct IcmpPacket<'a> {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: u16,
    pub identifier: u16,
    pub sequence: u16,
    pub payload: &'a [u8],
}

impl<'a> IcmpPacket<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let icmp_type = data[0];
        let code = data[1];
        let checksum = u16::from_be_bytes([data[2], data[3]]);
        let identifier = u16::from_be_bytes([data[4], data[5]]);
        let sequence = u16::from_be_bytes([data[6], data[7]]);

        Some(Self {
            icmp_type,
            code,
            checksum,
            identifier,
            sequence,
            payload: &data[8..],
        })
    }

    pub fn build_echo_request(id: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
        let total_len = 8 + payload.len();
        let mut data = Vec::with_capacity(total_len);
        data.push(ICMP_TYPE_ECHO_REQUEST);
        data.push(0); // Code 0
        data.extend_from_slice(&[0, 0]); // Checksum placeholder
        data.extend_from_slice(&id.to_be_bytes());
        data.extend_from_slice(&seq.to_be_bytes());
        data.extend_from_slice(payload);

        let csum = Ipv4Packet::calculate_checksum(&data);
        data[2..4].copy_from_slice(&csum.to_be_bytes());
        data
    }

    pub fn build_echo_reply(id: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
        let total_len = 8 + payload.len();
        let mut data = Vec::with_capacity(total_len);
        data.push(ICMP_TYPE_ECHO_REPLY);
        data.push(0); // Code 0
        data.extend_from_slice(&[0, 0]); // Checksum placeholder
        data.extend_from_slice(&id.to_be_bytes());
        data.extend_from_slice(&seq.to_be_bytes());
        data.extend_from_slice(payload);

        let csum = Ipv4Packet::calculate_checksum(&data);
        data[2..4].copy_from_slice(&csum.to_be_bytes());
        data
    }
}
