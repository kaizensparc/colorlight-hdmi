//! https://www.kernel.org/doc/html/v4.17/media/uapi/v4l/pixfmt-yuyv.html
//!
//! V4L2_PIX_FMT_YUYV — Packed format with 1/2 horizontal chroma resolution, also known as YUV 4:2:2
//! Description
//!
//! In this format each four bytes is two pixels. Each four bytes is two Y's, a Cb and a Cr. Each Y goes to one of the pixels, and the Cb and Cr belong to both pixels. As you can see, the Cr and Cb components have half the horizontal resolution of the Y component. V4L2_PIX_FMT_YUYV is known in the Windows environment as YUY2.
//!
//! Example 2.19. V4L2_PIX_FMT_YUYV 4 × 4 pixel image
//!
//! Byte Order. Each cell is one byte.
//! start + 0:	Y'00	Cb00	Y'01	Cr00	Y'02	Cb01	Y'03	Cr01
//! start + 8:	Y'10	Cb10	Y'11	Cr10	Y'12	Cb11	Y'13	Cr11
//! start + 16:	Y'20	Cb20	Y'21	Cr20	Y'22	Cb21	Y'23	Cr21
//! start + 24:	Y'30	Cb30	Y'31	Cr30	Y'32	Cb31	Y'33	Cr31
//!
//! Color Sample Location.
//!     0	 	1	 	2	 	3
//! 0	Y	C	Y	 	Y	C	Y
//! 1	Y	C	Y	 	Y	C	Y
//! 2	Y	C	Y	 	Y	C	Y
//! 3	Y	C	Y	 	Y	C	Y

use rayon::prelude::*;

/// Copies an input buffer of format YUYV422 to the output buffer
/// in the format of RGB24
#[inline]
pub fn yuv422_to_rgb24(in_buf: &[u8], out_buf: &mut [u8]) {
    debug_assert!(out_buf.len() as f32 == in_buf.len() as f32 * 1.5);

    in_buf
        .par_chunks_exact(4) // FIXME: use par_array_chunks() when stabalized (https://github.com/rayon-rs/rayon/pull/789)
        .zip(out_buf.par_chunks_exact_mut(6))
        .for_each(|(ch, out)| {
            let y1 = ch[0];
            let y2 = ch[2];
            let cb = ch[1];
            let cr = ch[3];

            let (r, g, b) = ycbcr_to_rgb(y1, cb, cr);

            out[0] = b;
            out[1] = g;
            out[2] = r;

            let (r, g, b) = ycbcr_to_rgb(y2, cb, cr);

            out[3] = b;
            out[4] = g;
            out[5] = r;
        });
}

// COLOR CONVERSION: https://stackoverflow.com/questions/28079010/rgb-to-ycbcr-using-simd-vectors-lose-some-data

fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
    // let ycbcr = vec![y as f32, cb as f32 - 128.0f32, cr as f32 - 128.0f32, 0.0];

    // rec 709: https://mymusing.co/bt-709-yuv-to-rgb-conversion-color/
    // let r = (ycbcr * f32x4::from_array([1.0, 0.00000, 1.5748, 0.0])).horizontal_sum();
    // let g = (ycbcr * f32x4::from_array([1.0, -0.187324, -0.468124, 0.0])).horizontal_sum();
    // let b = (ycbcr * f32x4::from_array([1.0, 1.8556, 0.00000, 0.0])).horizontal_sum();
    let r = (y as f32) + 1.5748f32 * (cr as f32 - 128.0f32);
    let g = (y as f32) - 0.187324f32 * (cb as f32 - 128.0f32) - 0.468124f32 * (cr as f32);
    let b = (y as f32) + 1.8556f32 * (cb as f32 - 128.0f32);

    (clamp(r), clamp(g), clamp(b))
}

fn clamp(val: f32) -> u8 {
    if val < 0.0 {
        0
    } else if val > 255.0 {
        255
    } else {
        val.round() as u8
    }
}
