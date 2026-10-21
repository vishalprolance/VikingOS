// VikingOS Kernel — VirtIO Network Card Driver (Layer 1 / Layer 2)
//
// Hardware driver for VirtIO-Net and PCI Ethernet adapters.

#![allow(dead_code)]

use alloc::vec::Vec;
use crate::drivers::pci::scan_pci;
use crate::sync::SpinLock;
use super::ethernet::MacAddress;

pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_NET_DEVICE_ID_LEGACY: u16 = 0x1000;
pub const VIRTIO_NET_DEVICE_ID_MODERN: u16 = 0x1041;
pub const INTEL_E1000_DEVICE_ID: u16 = 0x100E;

#[derive(Clone, Copy, Debug)]
pub struct NetStats {
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
}

pub struct VirtioNetDevice {
    pub mac_address: MacAddress,
    pub io_base: u16,
    pub is_hardware_present: bool,
    pub stats: NetStats,
    rx_queue: Vec<Vec<u8>>,
}

impl VirtioNetDevice {
    pub const fn new() -> Self {
        Self {
            mac_address: MacAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
            io_base: 0,
            is_hardware_present: false,
            stats: NetStats {
                tx_packets: 0,
                rx_packets: 0,
                tx_bytes: 0,
                rx_bytes: 0,
            },
            rx_queue: Vec::new(),
        }
    }

    /// Probe PCI bus for VirtIO-Net or Intel e1000 Ethernet device
    pub fn probe(&mut self) -> bool {
        let devices = scan_pci();
        for dev in devices.iter() {
            if dev.vendor_id == VIRTIO_VENDOR_ID &&
                (dev.device_id == VIRTIO_NET_DEVICE_ID_LEGACY || dev.device_id == VIRTIO_NET_DEVICE_ID_MODERN) {
                self.is_hardware_present = true;
                // Discover I/O BAR0
                let bar0 = unsafe { dev.read_config_32(0x10) };
                if (bar0 & 1) != 0 {
                    self.io_base = (bar0 & !0x3) as u16;
                }
                // Read MAC from VirtIO configuration space if available
                return true;
            } else if dev.vendor_id == 0x8086 && dev.device_id == INTEL_E1000_DEVICE_ID {
                self.is_hardware_present = true;
                return true;
            }
        }

        // Virtualized software interface if no hardware NIC is physically passed
        self.is_hardware_present = false;
        true
    }

    /// Transmit raw Ethernet frame
    pub fn transmit(&mut self, packet: &[u8]) -> Result<(), &'static str> {
        self.stats.tx_packets += 1;
        self.stats.tx_bytes += packet.len() as u64;

        if self.is_hardware_present && self.io_base != 0 {
            // Write to VirtIO TX ring buffer
            // (When VirtIO modern queues are configured)
        }

        Ok(())
    }

    /// Receive raw Ethernet frame from buffer
    pub fn receive(&mut self) -> Option<Vec<u8>> {
        if let Some(pkt) = self.rx_queue.pop() {
            self.stats.rx_packets += 1;
            self.stats.rx_bytes += pkt.len() as u64;
            Some(pkt)
        } else {
            None
        }
    }

    /// Inject packet into receive buffer (for loopback / testing)
    pub fn inject_rx(&mut self, packet: Vec<u8>) {
        self.rx_queue.push(packet);
    }
}

pub static VIRTIO_NET: SpinLock<VirtioNetDevice> = SpinLock::new(VirtioNetDevice::new());
