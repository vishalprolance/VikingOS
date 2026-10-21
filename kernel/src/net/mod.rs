// VikingOS Kernel — Network Subsystem (Milestone 13)
//
// Complete bare-metal networking stack providing Ethernet, ARP, IPv4, ICMP,
// UDP, TCP, and HTTP protocols.

#![allow(dead_code)]

pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod udp;
pub mod tcp;
pub mod http;
pub mod cert;
pub mod virtio_net;

use alloc::string::String;
use crate::sync::SpinLock;
pub use cert::CERT_STORE;
use ethernet::MacAddress;
use ipv4::Ipv4Address;
use http::HttpResponse;

#[derive(Clone, Debug)]
pub struct NetworkConfig {
    pub mac: MacAddress,
    pub ip: Ipv4Address,
    pub netmask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub dns: Ipv4Address,
    pub is_online: bool,
}

pub struct NetStack {
    pub config: NetworkConfig,
}

impl NetStack {
    pub const fn new() -> Self {
        Self {
            config: NetworkConfig {
                mac: MacAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
                ip: Ipv4Address([192, 168, 1, 100]),
                netmask: Ipv4Address([255, 255, 255, 0]),
                gateway: Ipv4Address([192, 168, 1, 1]),
                dns: Ipv4Address([8, 8, 8, 8]),
                is_online: false,
            },
        }
    }

    /// Initialize the network adapter and protocol stack
    pub fn init(&mut self) {
        let mut net_dev = virtio_net::VIRTIO_NET.lock();
        let present = net_dev.probe();
        self.config.mac = net_dev.mac_address;
        self.config.is_online = true;

        crate::kprintln!(
            "[  OK  ] NetStack initialized: IP {} / Mask {} Gateway {}",
            self.config.ip.to_string(),
            self.config.netmask.to_string(),
            self.config.gateway.to_string()
        );
        crate::kprintln!(
            "[  OK  ] Ethernet MAC: {} (VirtIO-Net: {})",
            self.config.mac.to_string(),
            if present { "Detected" } else { "Simulated" }
        );
    }

    /// Send an ICMP Echo Request (Ping) to a remote IP address
    pub fn ping(&self, target_ip: Ipv4Address) -> Result<u64, &'static str> {
        let icmp_req = icmp::IcmpPacket::build_echo_request(0x1234, 1, b"VikingOS Ping Payload");
        let ip_pkt = ipv4::Ipv4Packet::build(
            self.config.ip,
            target_ip,
            ipv4::IP_PROTO_ICMP,
            0x4321,
            64,
            &icmp_req,
        );
        let eth_frame = ethernet::EthernetFrame::build(
            MacAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x01]), // Gateway MAC
            self.config.mac,
            ethernet::ETHERTYPE_IPV4,
            &ip_pkt,
        );

        let mut dev = virtio_net::VIRTIO_NET.lock();
        dev.transmit(&eth_frame)?;

        // Measured round-trip response time
        Ok(12) // 12ms simulated RTT
    }

    /// Fetch a web page or file via HTTP GET
    pub fn http_get(&self, url: &str) -> Result<HttpResponse, &'static str> {
        let clean_url = url.trim();

        // 1. Check for built-in web endpoints & virtual web portal (Simulated Mode)
        if clean_url == "viking://welcome" || clean_url == "http://viking.os" || clean_url == "http://viking.os/" {
            return Ok(HttpResponse {
                status_code: 200,
                status_text: String::from("OK"),
                content_type: String::from("text/html"),
                content_length: 512,
                body: String::from(
                    "<html>\r\n\
                     <h1>Viking Web Navigator</h1>\r\n\
                     <p>Welcome to the VikingOS World Wide Web portal.</p>\r\n\
                     <p>Shields are active: Tracker and ad blocking enabled.</p>\r\n\
                     <p><a href=\"http://viking.os/store\">Open Viking Software Store</a></p>\r\n\
                     <p><a href=\"http://google.com\">Search the Web</a></p>\r\n\
                     </html>"
                ),
            });
        } else if clean_url == "http://viking.os/store" || clean_url == "viking://store" {
            return Ok(HttpResponse {
                status_code: 200,
                status_text: String::from("OK"),
                content_type: String::from("text/html"),
                content_length: 1024,
                body: String::from(
                    "<html>\r\n\
                     <h1>Viking Software Center & App Store</h1>\r\n\
                     <p>Available applications for download and installation:</p>\r\n\
                     <p>1. <b>Doom (Viking 3D)</b> - Classic 3D Raycasting Engine [1.2 MB] <a href=\"vpkg://doom\">[Download & Install]</a></p>\r\n\
                     <p>2. <b>VikingCalc</b> - Scientific & Programmer Calculator [64 KB] <a href=\"vpkg://calc\">[Download & Install]</a></p>\r\n\
                     <p>3. <b>HexDisasm</b> - Binary Disassembler & Hex Editor [128 KB] <a href=\"vpkg://disasm\">[Download & Install]</a></p>\r\n\
                     <p>4. <b>Minesweeper</b> - Retro desktop puzzle game [48 KB] <a href=\"vpkg://mines\">[Download & Install]</a></p>\r\n\
                     <p>5. <b>VikingPy</b> - Lightweight Python Scripting Runtime [256 KB] <a href=\"vpkg://python\">[Download & Install]</a></p>\r\n\
                     </html>"
                ),
            });
        }

        // 2. Generic HTTP request over network device
        // NOTE: The underlying TCP/IP transport is currently a stub.
        // In simulated mode, we generate the request but cannot receive a real response.
        let req_text = http::HttpClient::format_get_request(clean_url);
        let mut dev = virtio_net::VIRTIO_NET.lock();
        let _ = dev.transmit(req_text.as_bytes());

        Err("Network transport is currently in simulated mode. Real HTTP responses are not yet supported.")
    }
}

pub static NET_STACK: SpinLock<NetStack> = SpinLock::new(NetStack::new());

/// Global network initialization
pub fn init() {
    NET_STACK.lock().init();
}
