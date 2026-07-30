use atelier_image_analysis::{ImageAnalysisError, ImageAnalysisResult};
use image::{DynamicImage, Rgb, RgbImage, imageops};

pub const DBRATING_INPUT_SIZE: u32 = 384;
pub const WD_INPUT_SIZE: u32 = 448;

pub fn decode_rgb(bytes: &[u8]) -> ImageAnalysisResult<RgbImage> {
    image::load_from_memory(bytes)
        .map(|image| composite_white(&image))
        .map_err(|error| ImageAnalysisError::inference(format!("failed to decode image: {error}")))
}

fn composite_white(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, alpha] = rgba.get_pixel(x, y).0;
        let alpha = u16::from(alpha);
        let blend = |channel: u8| {
            u8::try_from((u16::from(channel) * alpha + 255 * (255 - alpha) + 127) / 255)
                .unwrap_or(255)
        };
        Rgb([blend(r), blend(g), blend(b)])
    })
}

pub fn dbrating_tensor(image: &RgbImage) -> Vec<f32> {
    let resized = imageops::resize(
        image,
        DBRATING_INPUT_SIZE,
        DBRATING_INPUT_SIZE,
        imageops::FilterType::Triangle,
    );
    let plane = (DBRATING_INPUT_SIZE * DBRATING_INPUT_SIZE) as usize;
    let mut tensor = vec![0.0; plane * 3];
    for (index, pixel) in resized.pixels().enumerate() {
        for channel in 0..3 {
            tensor[channel * plane + index] = f32::from(pixel.0[channel]) / 127.5 - 1.0;
        }
    }
    tensor
}

pub fn wd_tensor(image: &RgbImage) -> Vec<f32> {
    let side = image.width().max(image.height());
    let mut canvas = RgbImage::from_pixel(side, side, Rgb([255, 255, 255]));
    imageops::overlay(
        &mut canvas,
        image,
        i64::from((side - image.width()) / 2),
        i64::from((side - image.height()) / 2),
    );
    let resized = imageops::resize(
        &canvas,
        WD_INPUT_SIZE,
        WD_INPUT_SIZE,
        imageops::FilterType::CatmullRom,
    );
    let mut tensor = Vec::with_capacity((WD_INPUT_SIZE * WD_INPUT_SIZE * 3) as usize);
    for pixel in resized.pixels() {
        tensor.push(f32::from(pixel.0[2]));
        tensor.push(f32::from(pixel.0[1]));
        tensor.push(f32::from(pixel.0[0]));
    }
    tensor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dbrating_tensor_is_normalized_nchw() {
        let image = RgbImage::from_pixel(8, 8, Rgb([255, 128, 0]));
        let tensor = dbrating_tensor(&image);
        let plane = (DBRATING_INPUT_SIZE * DBRATING_INPUT_SIZE) as usize;

        assert_eq!(tensor.len(), plane * 3);
        assert!((tensor[0] - 1.0).abs() < 0.001);
        assert!(tensor[plane].abs() < 0.01);
        assert!((tensor[plane * 2] + 1.0).abs() < 0.001);
    }

    #[test]
    fn wd_tensor_is_bgr_nhwc_and_white_padded() {
        let image = RgbImage::from_pixel(4, 8, Rgb([10, 20, 30]));
        let tensor = wd_tensor(&image);

        assert_eq!(tensor.len(), (WD_INPUT_SIZE * WD_INPUT_SIZE * 3) as usize);
        assert_eq!(&tensor[..3], &[255.0, 255.0, 255.0]);
        let center = ((WD_INPUT_SIZE * WD_INPUT_SIZE / 2 + WD_INPUT_SIZE / 2) * 3) as usize;
        assert!((tensor[center] - 30.0).abs() < 2.0);
        assert!((tensor[center + 2] - 10.0).abs() < 2.0);
    }
}
