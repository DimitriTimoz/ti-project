use core::panic;
use std::io::Read;

use image::{DynamicImage, RgbImage};
use image::codecs::png::PngDecoder;

fn get_bit(data: &[u8], idx: usize) -> Option<bool> {
    let byte_idx = idx / 8;
    let bit_idx = idx % 8;
    if byte_idx < data.len() {
        Some(data[byte_idx] & (1 << bit_idx) != 0)
    } else {
        None
    }
}

fn store_data_in_image_lsb(image: &mut RgbImage, data: &[u8]) {
    let mut data2 = Vec::new();
    data2.extend_from_slice(&(data.len() as u32).to_be_bytes());
    data2.extend_from_slice(data);
    let data = data2;

    let bytes_to_store = data.len();
    let image_bytes = image.len();
    let bits_per_byte = (bytes_to_store*8)/image_bytes + 1;    
    println!("Have to use {} LSB bits per byte", bits_per_byte);
    if bits_per_byte > 8 {
        panic!("Image is too small to store the data");
    }
    
    let mut idx = 0;
    'outer: for (_x, _y, rgb) in image.enumerate_pixels_mut() {
        for color in &mut rgb.0 {
            for i in 0..bits_per_byte {
                let Some(bit) = get_bit(&data, idx) else {break 'outer};
                let mask = 1 << i;
                *color = (*color & !mask) | (bit as u8) << i;
                idx += 1;
            }
        }
    }
}

fn read_data_in_image_lsb(image: &RgbImage, data: &mut Vec<u8>, lsb: u8) {
    let mut buffer: u8 = 0;
    let mut message_length: Option<u32> = None;
    let mut idx = 0;
    // First read the message length
    'outer: for (_x, _y, rgb) in image.enumerate_pixels() {
        for color in &rgb.0 {
            for i in 0..lsb {
                let bit = (color >> i) & 1;
                buffer |= bit << idx;
                idx += 1;
                if idx == 8 {
                    if message_length.is_none() && data.len() == 4 {
                        data.push(buffer);
                        message_length = Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
                        println!("Message length: {:?}", message_length);
                        data.clear();
                        data.reserve(message_length.unwrap() as usize);
                    } else {
                        data.push(buffer);
                        if Some(data.len() as u32) == message_length {
                            break 'outer;
                        }
                    }
                    buffer = 0;
                    idx = 0;
                }
            }
        }
    }

}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = std::io::BufReader::new(std::fs::File::open("rick.png").unwrap());

    let decoder = PngDecoder::new(&mut reader)?;
    let img = DynamicImage::from_decoder(decoder)?;
    let mut image_buffer = img.to_rgb8();
    
    let data = "Hello, World!".as_bytes().to_vec();
    store_data_in_image_lsb(&mut image_buffer, &data);
    
    let new_image = DynamicImage::ImageRgb8(image_buffer);
    new_image.save("output.png")?;

    let mut reader = std::io::BufReader::new(std::fs::File::open("output.png").unwrap());
    let decoder = PngDecoder::new(&mut reader)?;
    let img = DynamicImage::from_decoder(decoder)?;
    let image_buffer = img.to_rgb8();
    let mut read_data = Vec::new();
    read_data_in_image_lsb(&image_buffer, &mut read_data, 1);
    println!("{:?}", read_data.len());
    println!("read_data: {:?}", read_data);
    println!("stored data: {:?}", data);
    let message = String::from_utf8_lossy(&read_data).into_owned();
    println!("{:?}", message);
    
    
    
    Ok(())
}
