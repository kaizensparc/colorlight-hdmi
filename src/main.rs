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

fn resize_image_with_cropping(
    mut src_view: fast_image_resize::DynamicImageView,
    dst_width: std::num::NonZeroU32,
    dst_height: std::num::NonZeroU32,
) -> fast_image_resize::Image {
    // Set cropping parameters
    src_view.set_crop_box_to_fit_dst_size(dst_width, dst_height, None);

    // Create container for data of destination image
    let mut dst_image = fast_image_resize::Image::new(dst_width, dst_height, src_view.pixel_type());
    // Get mutable view of destination image data
    let mut dst_view = dst_image.view_mut();

    // Create Resizer instance and resize source image
    // into buffer of destination image
    let mut resizer = fast_image_resize::Resizer::new(fast_image_resize::ResizeAlg::Convolution(
        fast_image_resize::FilterType::Lanczos3,
    ));
    resizer.resize(&src_view, &mut dst_view).unwrap();

    dst_image
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

        let (res_x, res_y) = (256u16, 64u16);
        let mut image = fast_image_resize::Image::from_vec_u8(
            std::num::NonZeroU32::new(fmt.width).unwrap(),
            std::num::NonZeroU32::new(fmt.height).unwrap(),
            rgb24.clone(),
            fast_image_resize::PixelType::U8x3,
        )
        .unwrap();

        // Linearize colospace before resizing
        let srgb_to_linear = fast_image_resize::create_srgb_mapper();
        srgb_to_linear
            .forward_map_inplace(&mut image.view_mut())
            .unwrap();
        let mut image = resize_image_with_cropping(
            image.view(),
            std::num::NonZeroU32::new(res_x as u32).unwrap(),
            std::num::NonZeroU32::new(res_y as u32).unwrap(),
        );
        srgb_to_linear
            .backward_map_inplace(&mut image.view_mut())
            .unwrap();
        let image = image.buffer();

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
            iface.send(&frame).expect("Could not send row");
        }
        // Wait a little bit before displaying the frames so that the FPGA can
        // store the last row in buffer, to avoid flickering
        std::thread::sleep(std::time::Duration::from_millis(5));

        // Finally, display it!
        let disp_frame = encode_disp_frame(0x03);
        iface.send(&disp_frame).expect("Could not send row");
    }
}
