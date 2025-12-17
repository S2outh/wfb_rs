use clap::Parser;
use wfb_rs::{common::utils, Receiver};

use std::time::Duration;
use std::net::UdpSocket;
use std::sync::mpsc::channel;
use std::thread;

/// Receiving side of wfb_rs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Magic number to identify the device
    #[arg(short = 'm', long, default_value_t = 0x57627273)]
    magic: u32,

    /// Forwarding Address
    #[arg(short = 'c', long, default_value = "127.0.0.1")]
    client_address: String,

    /// Forwarding Port
    #[arg(short = 'u', long, default_value_t = 5600)]
    client_port: u16,

    /// Listening Port
    #[arg(short = 'p', long, default_value_t = 0)]
    radio_port: u16,

    /// Link ID
    #[arg(short = 'i', long, default_value_t = 7669206)]
    link_id: u32,

    /// Log Interval
    #[arg(short='l', long, default_value = "1000", value_parser = parse_duration)]
    log_interval: Duration,

    /// Wifi Card setup (channel 149, monitor mode)
    #[arg(short='s', long, default_value_t = false)]
    wifi_setup: bool,

    /// Wifi Device
    #[arg(required = true, num_args = 1..)]
    wifi_devices: Vec<String>
}

fn parse_duration(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let milliseconds = arg.parse()?;
    Ok(std::time::Duration::from_millis(milliseconds))
}

fn main() {
    let args = Args::parse();

    println!("{:?}", args);

    if args.wifi_setup {
        for wifi in &args.wifi_devices {
            utils::set_monitor_mode(wifi.as_str()).unwrap();
        }
    }

    let rx = Receiver::new(
        args.magic,
        args.radio_port,
        args.link_id,
        args.wifi_devices,
    ).unwrap();

    run(rx,
        args.client_address,
        args.client_port,
        args.log_interval
    ).unwrap();
}

pub fn run(mut rx: Receiver,
    client_address: String,
    client_port: u16,
    log_interval: Duration)
    -> Result<(), Box<dyn std::error::Error>> {

    let udp_socket = UdpSocket::bind("0.0.0.0:0")?; // Bind to any available port
    
    let compound_output_address = format!("{}:{}", client_address, client_port);
    udp_socket.connect(&compound_output_address)?;
    
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

    loop {
        let (decoded_data, received_bytes) = rx.recv()?;
        received_bytes_s.send(received_bytes)?;

        for udp_pkg in decoded_data {
            match udp_socket.send(&udp_pkg) {
                Err(e) => {
                    eprintln!("Error forwarding packet: {}", e);
                }
                Ok(sent) => {
                    sent_bytes_s.send(sent as u32)?;
                }
            }
        }
    }
}
