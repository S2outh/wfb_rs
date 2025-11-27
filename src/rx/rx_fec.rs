use raptorq::{SourceBlockDecoder, EncodingPacket};
use std::iter::once;
use std::collections::{HashMap, HashSet};

use crate::common::fec;

#[cfg(feature = "dynamic")]
use crate::common::fec::header::FecHeader;

pub(super) struct RXFec {
    fec_decoders: HashMap<u8, SourceBlockDecoder>,
    decoded_blocks: HashSet<u8>,
}

impl RXFec {
    pub fn new() -> Self {
        Self {
            fec_decoders: HashMap::new(),
            decoded_blocks: HashSet::new(),
        }
    }
    #[cfg(feature = "dynamic")]
    fn build_packets(&self, mut decoded_data: Vec<u8>) -> Option<Vec<Vec<u8>>> {
        use std::mem::size_of;

        let Some(num_pkgs_lim) = decoded_data.pop() else { return None };
        if decoded_data.len() < num_pkgs_lim as usize * size_of::<u16>() { return None };
        let indices_start_index = decoded_data.len() - num_pkgs_lim as usize * size_of::<u16>();
        let pkg_indices: Vec<_> = decoded_data[indices_start_index..]
            .chunks(size_of::<u16>())
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let mut packets = Vec::new();
        for i in pkg_indices.windows(2) {
            let (start, end) = (i[0] as usize, i[1] as usize);
            packets.push(decoded_data[start..end].to_vec());
        }
        Some(packets)
    }
    #[cfg(not(feature = "dynamic"))]
    fn build_packets(&self, decoded_data: Vec<u8>) -> Option<Vec<Vec<u8>>> {
        let mut packets = Vec::new();
        for i in (0..decoded_data.len()).step_by(1200).collect::<Vec<_>>().windows(2) { //TODO
            let (start, end) = (i[0] as usize, i[1] as usize);
            packets.push(decoded_data[start..end].to_vec());
        }
        Some(packets)
    }
    pub fn process_fec_packet(
        &mut self,
        packet: &[u8],
    ) -> Option<Vec<Vec<u8>>> {

        // decoding fec header, returning the raw data if none is found
        #[cfg(feature = "dynamic")]
        let Some((fec_header, packet)) = FecHeader::from_bytes(packet) else {
            return None;
        };

        // get block id:
        let block_id = packet.get(0)?;

        // Check if we've already successfully decoded this block
        if self.decoded_blocks.contains(block_id) {
            // Already decoded this block, ignore this packet
            return None;
        }

        // Get or create decoder for this block
        if !self.fec_decoders.contains_key(block_id) {
            // Create ObjectTransmissionInformation with proper parameters
            // (transfer_length, symbol_size, sub_symbol_size, source_symbols, repair_symbols)

            #[cfg(feature = "dynamic")]
            let (config, padding) = fec::get_raptorq_oti(fec_header.block_size, fec_header.packet_size);
            #[cfg(not(feature = "dynamic"))]
            let (config, padding) = fec::get_raptorq_oti(9600, 800); //TODO
            self.fec_decoders
                .insert(*block_id, SourceBlockDecoder::new(*block_id, &config, config.transfer_length() + padding));
        }

        let decoder = self.fec_decoders.get_mut(block_id).unwrap();
        
        let packet = EncodingPacket::deserialize(packet);

        // add packet to decoder
        // Try to decode with current packets
        if let Some(decoded_data) = decoder.decode(once(packet)) {
            // Successfully decoded! Get the original udp packages:
            let packets = self.build_packets(decoded_data);

            // Clean up
            self.fec_decoders.remove(block_id);
            self.decoded_blocks.insert(*block_id);

            return packets;
        }

        // Clean up old decoders to prevent memory leak
        // Remove decoders older than current block_id - 64
        let cleanup_limit = 64;
        let cleanup_threshold_high = block_id.wrapping_add(cleanup_limit);
        let cleanup_threshold_low = block_id.wrapping_sub(cleanup_limit);
        let condition: Box<dyn Fn(u8) -> bool> = if cleanup_threshold_high > cleanup_threshold_low {
            Box::new(|a| cleanup_threshold_low < a && a < cleanup_threshold_high)
        } else {
            Box::new(|a| cleanup_threshold_low < a || a < cleanup_threshold_high)
        };
        self.fec_decoders.retain(|&k, _| condition(k));
        // Also clean up decoded blocks tracker
        self.decoded_blocks.retain(|&k| condition(k));

        return None; // Need more packets
    }
}

