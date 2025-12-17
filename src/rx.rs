mod rx_hardware_interface;
mod rx_fec;

use rx_hardware_interface::RXHwInt;
use rx_fec::RXFec;
use crate::common::magic_header::MagicHeader;

pub struct Receiver {
    rxs: Vec<RXHwInt>,
    fec: RXFec,
    magic_header: MagicHeader,
}

impl Receiver {
    pub fn new(
        magic: u32,
        radio_port: u16,
        link_id: u32,
        wifi_devices: Vec<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let channel_id = link_id << 8 | radio_port as u32;

        let rxs: Vec<RXHwInt> = wifi_devices
            .into_iter()
            .map(|dev| RXHwInt::new(dev, channel_id))
            .collect::<Result<_, _>>()?;


        let fec = RXFec::new();

        let magic_header = MagicHeader::new(magic);

        Ok(Self {
            rxs,
            fec,
            magic_header,
        })
    }

    pub fn recv(&mut self) -> Result<(Vec<Vec<u8>>, u32), Box<dyn std::error::Error>> {
        let mut received_bytes = 0;
        loop {
            for rx in &mut self.rxs {
                let Some(raw_packet) = rx.receive_packet()? else { continue; };
                received_bytes += raw_packet.len() as u32;

                let Some((fec_pkg, wfb_packet)) = self.magic_header.from_bytes(&raw_packet) else { continue; };
                
                let decoded_data = if fec_pkg {
                    let Some(decoded_data) = self.fec.process_fec_packet(&wfb_packet) else { continue; };
                    decoded_data
                } else {
                    vec![wfb_packet.to_vec()]
                };

                return Ok((decoded_data, received_bytes));
            }
        }
    }
}
