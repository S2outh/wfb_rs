
use raptorq::ObjectTransmissionInformation;


pub fn get_raptorq_oti(block_size: u16, wifi_packet_size: u16) -> (ObjectTransmissionInformation, u64) {
    let config = ObjectTransmissionInformation::with_defaults(block_size as u64, wifi_packet_size);
    let padding = config.symbol_size() as u64 - config.transfer_length() % config.symbol_size() as u64;
    (config, if padding == config.symbol_size() as u64 { 0 } else { padding })
}

#[cfg(feature = "dynamic")]
pub mod header {
    use std::mem::size_of;

    // FEC Header constants and structures
    const FEC_HEADER_SIZE: usize = size_of::<FecHeader>();
    
    #[derive(Debug, Clone, Copy)]
    pub struct FecHeader {
        pub block_size: u16,    // 2 bytes - the total size of the current fec block in bytes
        pub packet_size: u16,    // 2 bytes - the size of the wifi packet in bytes
    }
    
    impl FecHeader {
        pub fn new(block_size: u16, packet_size: u16) -> Self {
            Self {
                block_size,
                packet_size,
            }
        }
    
        pub fn to_bytes(&self) -> [u8; FEC_HEADER_SIZE] {
            let mut bytes = [0u8; FEC_HEADER_SIZE];
            bytes[0..2].copy_from_slice(&self.block_size.to_le_bytes());
            bytes[2..4].copy_from_slice(&self.packet_size.to_le_bytes());
            bytes
        }
    
        #[cfg(feature = "receiver")]
        pub fn from_bytes(bytes: &[u8]) -> Option<(Self, &[u8])> {
            if bytes.len() < FEC_HEADER_SIZE {
                return None;
            }
    
            let block_size = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
            let packet_size = u16::from_le_bytes(bytes[2..4].try_into().unwrap());
    
            Some((Self {
                block_size,
                packet_size
            }, &bytes[FEC_HEADER_SIZE..]))
        }
    }
}
