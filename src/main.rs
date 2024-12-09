use image::{DynamicImage, RgbImage};
use image::codecs::png::PngDecoder;
use std::f64::consts::LOG2_E;

// Histogramme des LSB
fn analyze_lsb_histogram(image: &RgbImage, lsb_mask: u8) {
    let mut lsb_counts = [0u32; 2];
    for (_x, _y, rgb) in image.enumerate_pixels() {
        for &color in &rgb.0 {
            for i in 0..8-lsb_mask.leading_zeros() {
                let bit = (color >> i) & 1;
                lsb_counts[bit as usize] += 1;
            }
        }
    }

    let total = lsb_counts[0] + lsb_counts[1];
    println!("Histogramme des LSB :");
    println!("0 : {} ({:.2}%)", lsb_counts[0], lsb_counts[0] as f64 / total as f64 * 100.0);
    println!("1 : {} ({:.2}%)", lsb_counts[1], lsb_counts[1] as f64 / total as f64 * 100.0);
}

// Test du Chi-carré
fn perform_chi_squared_test(image: &RgbImage, lsb_mask: u8) {
    let mut lsb_counts = [0u32; 2];

    for (_x, _y, rgb) in image.enumerate_pixels() {
        for &color in &rgb.0 {
            for i in 0..8-lsb_mask.leading_zeros() {
                let bit = (color >> i) & 1;
                lsb_counts[bit as usize] += 1;
            }
        }
    }

    let total = (lsb_counts[0] + lsb_counts[1]) as f64;
    let expected = total / 2.0;

    let chi_squared = ((lsb_counts[0] as f64 - expected).powi(2) / expected)
                    + ((lsb_counts[1] as f64 - expected).powi(2) / expected);

    println!("\nTest du Chi-carré :");
    println!("Statistique Chi-carré : {:.2}", chi_squared);

    // Degré de liberté = catégories - 1 = 1
    // Valeur critique à alpha = 0.05 est 3.841
    if chi_squared > 3.841 {
        println!("La différence est statistiquement significative au niveau alpha=0.05");
    } else {
        println!("La différence n'est pas statistiquement significative au niveau alpha=0.05");
    }
}

// Entropie locale
fn safe_entropy(p: f64) -> f64 {
    if p > 0.0 {
        -p * p.ln() / LOG2_E
    } else {
        0.0
    }
}

fn compute_local_entropy(image: &RgbImage, lsb_mask: u8) {
    let window_size = 8;
    let lsb_bits = lsb_mask.count_ones() as usize; // nombre de bits LSB à utiliser

    let width = image.width() as usize;
    let height = image.height() as usize;

    let mut total_entropy = 0.0;
    let mut window_count = 0;

    for y in (0..height).step_by(window_size) {
        for x in (0..width).step_by(window_size) {
            let mut lsb_counts = [0u32; 2];
            for dy in 0..window_size {
                for dx in 0..window_size {
                    let px = x + dx;
                    let py = y + dy;
                    if px < width && py < height {
                        let pixel = image.get_pixel(px as u32, py as u32);
                        for &color in &pixel.0 {
                            for i in 0..lsb_bits {
                                let bit = (color >> i) & 1;
                                lsb_counts[bit as usize] += 1;
                            }
                        }
                    }
                }
            }

            let total = (lsb_counts[0] + lsb_counts[1]) as f64;
            if total > 0.0 {
                let p0 = lsb_counts[0] as f64 / total;
                let p1 = lsb_counts[1] as f64 / total;

                // Calcul direct de l'entropie en bits via log base 2
                // Entropie = -∑ p(i) * log2(p(i))
                let entropy =
                    if p0 > 0.0 { -p0 * p0.log2() } else { 0.0 } +
                    if p1 > 0.0 { -p1 * p1.log2() } else { 0.0 };

                total_entropy += entropy;
                window_count += 1;
            }
        }
    }

    if window_count > 0 {
        let average_entropy = total_entropy / window_count as f64;
        println!("\nEntropie locale :");
        println!("Entropie moyenne : {:.4} bits", average_entropy);
    } else {
        println!("\nAucune fenêtre valide n'a été trouvée pour le calcul de l'entropie.");
    }
}


fn get_bit(data: &[u8], idx: usize) -> Option<bool> {
    let byte_idx = idx / 8;
    let bit_idx = idx % 8;
    if byte_idx < data.len() {
        Some(data[byte_idx] & (1 << bit_idx) != 0)
    } else {
        None
    }
}

fn store_data_in_image_lsb(image: &mut RgbImage, data: &[u8]) -> usize {
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

    bits_per_byte
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
                    data.push(buffer);
                    if message_length.is_none() && data.len() == 4 {
                        message_length = Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
                        println!("Message length: {:?}", message_length);
                        data.clear();
                        data.reserve(message_length.unwrap() as usize);
                    }
                    if Some(data.len() as u32) == message_length {
                        break 'outer;
                    }
                    buffer = 0;
                    idx = 0;
                }
            }
        }
    }

}

fn create_lsb_only_image(image: &RgbImage, lsb_mask: u8, path: &str) {
    let mut new_image = RgbImage::new(image.width(), image.height());
    for (x, y, rgb) in image.enumerate_pixels() {
        let mut new_rgb = [0u8; 3];
        for (i, color) in rgb.0.iter().enumerate() {
            new_rgb[i] = color & lsb_mask;
            new_rgb[i] |= new_rgb[i] << lsb_mask.leading_zeros();
        }
        new_image.put_pixel(x, y, image::Rgb(new_rgb));
    }
    new_image.save(path).unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = std::io::BufReader::new(std::fs::File::open("rick.png").unwrap());

    let decoder = PngDecoder::new(&mut reader)?;
    let img = DynamicImage::from_decoder(decoder)?;
    let mut image_buffer = img.to_rgb8();
    println!("Image d'origine =====");
    analyze_lsb_histogram(&image_buffer, 0b111);
    perform_chi_squared_test(&image_buffer, 0b111);
    compute_local_entropy(&image_buffer, 0b111);
    create_lsb_only_image(&image_buffer, 0b1, "lsb_only_1bit.png");
    create_lsb_only_image(&image_buffer, 0b111, "lsb_only_3bits.png");

    let data = include_str!("the_hobbit.txt").as_bytes().to_vec();
    store_data_in_image_lsb(&mut image_buffer, &data);
    
    let new_image = DynamicImage::ImageRgb8(image_buffer);
    new_image.save("output.png")?;

    let mut reader = std::io::BufReader::new(std::fs::File::open("output.png").unwrap());
    let decoder = PngDecoder::new(&mut reader)?;
    let img = DynamicImage::from_decoder(decoder)?;
    let image_buffer = img.to_rgb8();
    let mut read_data = Vec::new();
    read_data_in_image_lsb(&image_buffer, &mut read_data, 3);
    let message = String::from_utf8_lossy(&read_data).into_owned();
    //println!("{:?}", message);
    
    println!("Image contenant le message =====");
    analyze_lsb_histogram(&image_buffer, 0b111);
    perform_chi_squared_test(&image_buffer, 0b111);
    compute_local_entropy(&image_buffer, 0b111);
    create_lsb_only_image(&image_buffer, 0b111, "lsb_hobbit.png");
    


    // Encode image into image
    let mut reader = std::io::BufReader::new(std::fs::File::open("rick.png").unwrap());
    let decoder = PngDecoder::new(&mut reader)?;
    let img = DynamicImage::from_decoder(decoder)?;
    let mut image_buffer = img.to_rgb8();
    let data = include_bytes!("image.png");
    let bits_per_byte = store_data_in_image_lsb(&mut image_buffer, data);
    create_lsb_only_image(&image_buffer, 0b1, "lsb_pirate.png");
    let new_image = DynamicImage::ImageRgb8(image_buffer);
    new_image.save("output2.png")?;

    // Decode image from image
    let mut reader = std::io::BufReader::new(std::fs::File::open("output2.png").unwrap());
    let decoder = PngDecoder::new(&mut reader)?;
    let img = DynamicImage::from_decoder(decoder)?;
    let image_buffer = img.to_rgb8();
    let mut read_data = Vec::new();
    read_data_in_image_lsb(&image_buffer, &mut read_data, bits_per_byte as u8);
    std::fs::write("image-decoded.png", read_data)?;
    
    Ok(())
}
