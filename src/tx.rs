mod tx_hardware_interface;
mod tx_fec;

use std::net::UdpSocket;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{io, thread};

use super::common::{hw_headers, magic_header, bandwidth::Bandwidth};

use tx_hardware_interface::TXHwInt;
use tx_fec::TXFec;
use magic_header::MagicHeader;

pub struct Transmitter {
    tx: TXHwInt,
    fec: Option<TXFec>,
    magic_header: MagicHeader,
}

impl Transmitter {
    pub fn new(
        magic: u32,
        radio_port: u8,
        link_id: u32,
        bandwidth: Bandwidth,
        short_gi: bool,
        stbc: u8,
        ldpc: bool,
        mcs_index: u8,
        vht_mode: bool,
        vht_nss: u8,
        wifi_device: String,
        fec_disabled: bool,
        min_block_size: u16,
        wifi_packet_size: u16,
        redundant_pkgs: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let radiotap_header = hw_headers::get_radiotap_headers(
            stbc, ldpc, short_gi, bandwidth, mcs_index, vht_mode, vht_nss,
        );
        let link_id = link_id & 0xffffff;

        let channel_id = link_id << 8 | radio_port as u32;

        let tx = TXHwInt::new(wifi_device, radiotap_header, channel_id)?;

        let fec = if fec_disabled {
            None
        } else { Some(TXFec::new(
            min_block_size,
            wifi_packet_size,
            redundant_pkgs
        ))};

        let magic_header = if fec_disabled {
            MagicHeader::new(magic)
        } else {
            MagicHeader::new_fec(magic)
        };

        Ok(Self {
            tx,
            fec,
            magic_header,
        })
    }

    pub fn run(mut self, source_port: u16, buffer_r: usize, log_interval: Duration) -> Result<(), Box<dyn std::error::Error>> {

        let udp_socket = UdpSocket::bind(format!("0.0.0.0:{}", source_port))?;
        
        let (sent_bytes_s, sent_bytes_r) = channel();
        let (received_bytes_s, received_bytes_r) = channel();

        // start logtask
        thread::spawn(move || {
            loop {
                let (sent_packets, sent_bytes): (u32, u32) = sent_bytes_r.try_iter().fold((0, 0), |(count, sum), v| (count + 1, sum + v));
                let (received_packets, received_bytes): (u32, u32) = received_bytes_r.try_iter().fold((0, 0), |(count, sum), v| (count + 1, sum + v));
                println!(
                    "Packets R->T {}->{},\tBytes {}->{}",
                    received_packets,
                    sent_packets,
                    received_bytes,
                    sent_bytes,
                );
                thread::sleep(log_interval);
            }
        });

        let (packet_s, packet_r) = channel();

        // start sendtask
        thread::spawn(move || {
            loop {
                let udp_packet: Vec<u8> = packet_r.recv().expect("packet sender closed");
                let sent = self.send(&udp_packet);
                sent_bytes_s.send(sent as u32).unwrap();
            }
        });

        loop {
            let mut udp_recv_buffer = vec![0u8; buffer_r];
            let poll_result = udp_socket.recv(&mut udp_recv_buffer);

            match poll_result {
                Err(err) => match err.kind() {
                    io::ErrorKind::TimedOut => continue,
                    err => {
                        eprintln!("Error polling udp input: {}", err);
                        continue;
                    },
                },
                Ok(received) => {
                    if received == 0 {
                        //Empty packet
                        eprintln!("Empty packet");
                        continue;
                    }
                    if received == buffer_r {
                        eprintln!("Input packet seems too large");
                    }
                    
                    let udp_packet = udp_recv_buffer[..received].to_vec();

                    received_bytes_s.send(received as u32)?;

                    packet_s.send(udp_packet).expect("packet receiver closed");
                }
            }
        }
    }
    pub fn send(&mut self, packet: &[u8]) -> u32 {
        let block = if let Some(fec) = self.fec.as_mut() {
            if let Some(block) = fec.process_packet_fec(packet) {
                block
            } else {
                return 0;
            }
        } else {
            // if fec is disabled just send the raw block
            vec![packet.to_vec()]
        };

        let mut sent_bytes = 0;

        for wfb_packet in block.into_iter() {
            // add magic number
            let packet = [&self.magic_header.to_bytes(), &wfb_packet[..]].concat();
            // send via raw socket
            let sent = self.tx.send_packet(&packet).unwrap() as u32;
            if sent < packet.len() as u32 {
                eprintln!("socket dropped some bytes");
            }
            sent_bytes += sent as u32;
        }
        return sent_bytes;
    }
}
