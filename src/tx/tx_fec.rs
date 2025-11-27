use raptorq::SourceBlockEncoder;

use crate::common::fec;

#[cfg(feature = "dynamic")]
use crate::common::fec::header::FecHeader;

pub(super) struct TXFec {
    block_id: u8,
    pkg_indices: Vec<u16>,
    block_buffer: Vec<u8>,
    block_size: u16,
    wifi_packet_size: u16,
    redundant_pkgs: u32,
}

impl TXFec {
    pub fn new(block_size: u16, wifi_packet_size: u16, redundant_pkgs: u32) -> Self {
        Self {
            block_id: 0,
            pkg_indices: Vec::new(),
            block_buffer: Vec::new(),
            block_size,
            wifi_packet_size,
            redundant_pkgs
        }
    }
    #[cfg(feature = "dynamic")]
    fn build_block(&mut self) -> Vec<Vec<u8>> {
        use std::iter::once;

        // add udp package limiter info header (append it for performance)
        let udp_pkgs_header: Vec<_> = self.pkg_indices
            .iter()
            .map(|i| i.to_le_bytes())
            .chain(once((self.block_buffer.len() as u16).to_le_bytes()))
            .flatten()
            .chain(once(self.pkg_indices.len() as u8 + 1))
            .collect();

        // if block is full, return it
        let block_size = self.block_buffer.len() as u16 + udp_pkgs_header.len() as u16;
        let (config, padding) = fec::get_raptorq_oti(block_size, self.wifi_packet_size);
        self.block_buffer.extend(vec![0; padding as usize]);
        self.block_buffer.extend(udp_pkgs_header);
        let encoder = SourceBlockEncoder::new(self.block_id, &config, &self.block_buffer);

        let header = FecHeader::new(block_size, self.wifi_packet_size).to_bytes();
        let mut packets = vec![];
        packets.extend(encoder.source_packets());
        packets.extend(encoder.repair_packets(0, self.redundant_pkgs));
        packets
            .iter()
            .map(|e| [&header, &e.serialize()[..]].concat())
            .collect()
    }
    #[cfg(not(feature = "dynamic"))]
    fn build_block(&mut self) -> Vec<Vec<u8>> {
        // if block is full, return it
        let (config, padding) = fec::get_raptorq_oti(self.block_size, self.wifi_packet_size);
        assert_eq!(padding, 0, "in static mode the configured block size must be a multiple of the wifi packet size");
        let encoder = SourceBlockEncoder::new(self.block_id, &config, &self.block_buffer);

        let mut packets = vec![];
        packets.extend(encoder.source_packets());
        packets.extend(encoder.repair_packets(0, self.redundant_pkgs));
        packets
            .iter()
            .map(|e| e.serialize())
            .collect()
    }
    pub fn process_packet_fec(&mut self, packet: &[u8]) -> Option<Vec<Vec<u8>>> {
        // wait for block buffer to fill
        self.pkg_indices.push(self.block_buffer.len() as u16);
        self.block_buffer.extend_from_slice(packet);
        if self.block_buffer.len() < self.block_size as usize {
            return None;
        }

        let block = self.build_block();

        self.block_id = self.block_id.wrapping_add(1);
        self.block_buffer.clear();
        self.pkg_indices.clear();
        Some(block)
    }
}

