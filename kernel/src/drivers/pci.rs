// VikingOS Kernel — PCI Bus Driver
//
// Provides basic PCI scanning and device discovery capabilities.

#![allow(dead_code)]

use alloc::vec::Vec;
use crate::arch::io::{inl, outl};

/// PCI Configuration Space offsets.
pub const PCI_CONFIG_VENDOR_ID: u16 = 0x00;
pub const PCI_CONFIG_DEVICE_ID: u16 = 0x02;
pub const PCI_CONFIG_COMMAND: u16 = 0x04;
pub const PCI_CONFIG_STATUS: u16 = 0x06;
pub const PCI_CONFIG_CLASS: u16 = 0x08;
pub const PCI_CONFIG_BAR0: u16 = 0x10;

/// PCI Configuration Address Port.
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
/// PCI Configuration Data Port.
const PCI_CONFIG_DATA_PORT: u16 = 0xCFC;

pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
}

impl PciDevice {
    /// Read a 32-bit value from the PCI configuration space.
    pub unsafe fn read_config_32(&self, offset: u16) -> u32 {
        let address = (1 << 31)
            | ((self.bus as u32) << 16)
            | ((self.slot as u32) << 11)
            | ((self.function as u32) << 8)
            | (offset as u32 & !0xFC);

        unsafe {
            outl(PCI_CONFIG_ADDRESS, address);
            inl(PCI_CONFIG_DATA_PORT)
        }
    }

    /// Write a 32-bit value to the PCI configuration space.
    pub unsafe fn write_config_32(&self, offset: u16, value: u32) {
        let address = (1 << 31)
            | ((self.bus as u32) << 16)
            | ((self.slot as u32) << 11)
            | ((self.function as u32) << 8)
            | (offset as u32 & !0xFC);

        unsafe {
            outl(PCI_CONFIG_ADDRESS, address);
            outl(PCI_CONFIG_DATA_PORT, value);
        }
    }
}

/// Scans the PCI bus for devices.
pub fn scan_pci() -> Vec<PciDevice> {
    let mut devices = Vec::new();

    for bus in 0..=255 {
        for slot in 0..32 {
            for func in 0..8 {
                let vendor_id = unsafe { read_config_16(bus, slot, func, PCI_CONFIG_VENDOR_ID) };
                if vendor_id != 0xFFFF {
                    let device_id = unsafe { read_config_16(bus, slot, func, PCI_CONFIG_DEVICE_ID) };
                    devices.push(PciDevice {
                        bus,
                        slot,
                        function: func,
                        vendor_id,
                        device_id,
                    });
                }
            }
        }
    }
    devices
}

unsafe fn read_config_16(bus: u8, slot: u8, func: u8, offset: u16) -> u16 {
    let address = (1 << 31)
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | (offset as u32 & !0xFC);

    unsafe {
        outl(PCI_CONFIG_ADDRESS, address);
        (inl(PCI_CONFIG_DATA_PORT) & 0xFFFF) as u16
    }
}
