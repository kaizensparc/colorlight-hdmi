use v4l::buffer::Type;
use v4l::io::mmap::Stream;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;
use v4l::Device;
use v4l::FourCC;

mod yuv;

const RECV_MAC: [u8; 6] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
const SEND_MAC: [u8; 6] = [0x22, 0x22, 0x33, 0x44, 0x55, 0x66];
const DETECT_HDR: [u8; 2] = [0x07, 0x00];
const BRIGHT_HDR: [u8; 1] = [0x0A];
const DISP_HDR: [u8; 2] = [0x01, 0x07];

// Packet Format: (info based on mplayer ColorLight 5a-75 video output patch)
//
// 0x0101 Packet: (send first)
// 	- Data Length:     98
// 	- Ether Type:      0x0101 (have also seen 0x0100, 0x0104, 0x0107.
// 	- Data[0-end]:     0x00
//
// Row data packets: (send one packet for each row of display)
//    - Data Length:     (Row_Width * 3) + 7
// 	- Ether Type:      0x5500 + MSB of Row Number
// 	                     0x5500 for rows 0-255
// 	                     0x5501 for rows 256-511
// 	- Data[0]:         Row Number LSB
// 	- Data[1]:         MSB of pixel offset for this packet
// 	- Data[2]:         LSB of pixel offset for this packet
// 	- Data[3]:         MSB of pixel count in packet
// 	- Data[4]:         LSB of pixel count in packet
// 	- Data[5]:         0x08 - ?? unsure what this is
// 	- Data[6]:         0x80 - ?? unsure what this is
// 	- Data[7-end]:     RGB order pixel data
//
// Sample data packets seen in captures:
//         0  1  2  3  4  5  6
//   55 00 00 00 00 01 F1 00 00 (first 497 pixels on a 512 wide display)
//   55 00 00 01 F1 00 0F 00 00 (last 15 pixels on a 512 wide display)
//   55 00 00 00 00 01 20 08 88 (288 pixel wide display)
//   55 00 00 00 00 00 80 08 88 (128 pixel wide display)
//
//

fn encode_disp_frame(brightness: u8) -> Vec<u8> {
    let mut frame = vec![];
    frame.extend_from_slice(&RECV_MAC);
    frame.extend_from_slice(&SEND_MAC);
    frame.extend_from_slice(&DISP_HDR);
    frame.extend_from_slice(&[0u8; 11]);
    frame.extend_from_slice(&[brightness, 0x05, brightness, brightness, brightness]);
    frame.extend_from_slice(&[0u8; 73]);

    frame
}

fn encode_recv_frame() -> Vec<u8> {
    let mut recv_frame = vec![];
    recv_frame.extend_from_slice(&RECV_MAC);
    recv_frame.extend_from_slice(&SEND_MAC);
    recv_frame.extend_from_slice(&DETECT_HDR);
    recv_frame.extend_from_slice(&[0u8; 270]);

    recv_frame
}

fn encode_bright_frame(brightness: u8) -> Vec<u8> {
    let mut frame = vec![];
    frame.extend_from_slice(&RECV_MAC);
    frame.extend_from_slice(&SEND_MAC);
    frame.extend_from_slice(&BRIGHT_HDR);
    frame.extend_from_slice(&[brightness; 3]);
    frame.push(0xFF);
    frame.extend_from_slice(&[0u8; 60]);

    frame
}

/// Generate a test pattern, result is in RGB888 format
fn generate_test_pattern(size_x: usize, size_y: usize, it: u32) -> Vec<u8> {
    let mut pattern = std::vec::Vec::with_capacity(size_x * size_y * 3);
    for x in 0..size_x {
        for y in 0..size_y {
            pattern.extend_from_slice(if x + it as usize == y {
                &[0xFF; 3]
            } else {
                &[0u8; 3]
            });
        }
    }
    pattern
}

fn main() {
    // This part is very std/Hosted platform specific
    let lib = rawsock::open_best_library().expect("Could not open any packet capturing library");
    println!("Using socket packet capture library: {}", lib.version());
    let iface = "ens33";
    let mut iface = lib
        .open_interface(&iface)
        .expect("Could not open network interface");
    println!("Interface opened, data link: {}", iface.data_link());

    // Open HDMI capture device
    let mut dev = Device::new(0).expect("Failed to open device");

    // Set format
    let mut fmt = dev.format().expect("Failed to read format");
    fmt.width = 640;
    fmt.height = 480;
    fmt.fourcc = FourCC::new(b"YUYV");
    let fmt = dev.set_format(&fmt).expect("Failed to write format");
    println!("Format in use:\n{}", fmt);

    // Try to detect the colorlight card
    println!("Looking for a colorlight card");

    iface
        .send(&encode_recv_frame())
        .expect("Could not send discovery packet");

    loop {
        let packet = iface.receive().expect("Could not receive packet");
        // Check dst mac is ff:ff:ff:ff:ff:ff, src mac is RECV_MAC and frame header is 0x0805
        if packet.len() >= 112
            && packet.starts_with(&[0xffu8; 6])
            && packet[6..12].starts_with(&RECV_MAC)
            && packet[12..14].starts_with(&[0x08, 0x05])
        {
            let fw = format!("{}.{}", packet[15], packet[16]);
            let res_x = packet[34] as u16 * 256 + packet[35] as u16;
            let res_y = packet[36] as u16 * 256 + packet[37] as u16;
            let chain = packet[112];
            //println!("len: {}, packet: {:02X?}", packet.len(), packet);
            println!(
                "Detected colorlight card 5A, fw: {}, res: {}x{}, chain number: {}",
                fw, res_x, res_y, chain
            );
            break;
        }
    }

    // Set main brightness
    let bright_frame = encode_bright_frame(0x28);
    //println!("BRIGHT: {:02X?}", bright_frame);
    iface
        .send(&bright_frame)
        .expect("Could not send brightness packet");

    // Set stream
    let mut stream =
        Stream::new(&mut dev, Type::VideoCapture).expect("Failed to create buffer stream");

    loop {
        let (buf, meta) = stream.next().unwrap();
        println!(
            "Buffer size: {}, seq: {}, timestamp: {}",
            buf.len(),
            meta.sequence,
            meta.timestamp
        );
        let mut rgb24 = vec![0; (buf.len() as f32 * 1.5) as usize];
        yuv::yuv422_to_rgb24(buf, &mut rgb24);
        println!("RGB size: {}", rgb24.len());

        let (frame_x, frame_y) = (640usize, 480usize);
        let (res_x, res_y) = (128u16, 128u16);
        // Center of the image
        let (offset_x, offset_y) = (
            (frame_x - res_x as usize) / 2,
            (frame_y - res_y as usize) / 2,
        );
        let image: Vec<u8> = rgb24
            .into_iter()
            .enumerate()
            // We currently have 3 bytes per pixel, and 640x480 pixels
            // We want to take the 128x128 square on the upper left + pixel shift
            .filter(|&(pix, _)| {
                let pix_coord_x = (pix / 3) % frame_x;
                let pix_coord_y = (pix / 3) / frame_x;
                let in_col =
                    (pix_coord_x >= offset_x) && (pix_coord_x < (offset_x + res_x as usize));
                let in_row = (pix_coord_y >= offset_y) && (pix_coord_y < offset_y + res_y as usize);
                in_col && in_row
            })
            .map(|(_, v)| v)
            .collect();
        println!("Frame size: {}", image.len());

        // Now send the stream!

        let bright_frame = encode_bright_frame(0x28);
        iface
            .send(&bright_frame)
            .expect("Could not send brightness packet");

        for row in 0..res_y {
            let mut frame = vec![];
            frame.extend_from_slice(&RECV_MAC);
            frame.extend_from_slice(&SEND_MAC);
            frame.push(0x55);
            frame.extend_from_slice(&row.to_be_bytes());
            // Pixel offset :thinking:
            frame.extend_from_slice(&[0u8; 2]);
            frame.extend_from_slice(&res_x.to_be_bytes());
            frame.extend_from_slice(&[0x08, 0x88]);

            let pixel_start = (row * res_x * 3) as usize;
            let pixel_stop = ((row + 1) * res_x * 3) as usize;
            frame.extend_from_slice(&image[pixel_start..pixel_stop]);

            // Send it
            //println!("ROW {}: {:02X?}", row, frame);
            iface.send(&frame).expect("Could not send row");
        }

        // Finally, display it!
        let disp_frame = encode_disp_frame(0x03);
        //println!("DISP: {:02X?}", disp_frame);
        iface.send(&disp_frame).expect("Could not send row");

        //let mut previous_frame = std::time::Instant::now();
        //std::thread::sleep(std::time::Duration::from_secs(10));
    }

    //stream.stop();
}
