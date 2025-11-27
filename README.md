# wfb_rs
Rewriting wifibroadcast in Rust. This offers many improvements, like allowing modern and fast error correction algorithms that decrease packet loss.

## Features

### Forward Error Correction (FEC)
This implementation includes optional Forward Error Correction using the RaptorQ fountain code. FEC helps recover lost packets in wireless transmission, improving reliability especially in environments with interference or weak signal conditions.


#### Building the software

If you want to build both sender and receiver, you need to set the feature flag
```bash
cargo build --features=receiver --release
```

#### Using FEC
When built with FEC support, both transmitter and receiver will automatically use RaptorQ encoding/decoding:

- **Transmitter**: If enabled, it encodes each UDP packet into multiple FEC packets (default: 15 packets per block). Otherwise it adds radiotap and wifi headers and forwards tthe packets as they are.
- **Receiver**: Automatically detects if packets are FEC packets and attempts to decode them back to the original data. Otherwise raw udp traffic is forwarded

You can disable FEC at runtime using `./wfb_rs_tx --fec-disabled ...`

## Cross compiling for the raspi:

`cross build --features=receiver --release`

If you are strongly limited in storage space, you can optimize via upx, this will save about 2/3 of binary size

`upx --best --lzma target/aarch64-unknown-linux-gnu/debug/tx_cli`
