use ash::vk;
use gltf::{buffer::Data, image::Format};
use image::{DynamicImage, ImageBuffer, Pixel};
use thiserror::Error;

pub struct GpuImage {
    pub bytes: Vec<u8>,
    pub dims: glam::UVec3,
    pub format: vk::Format,
}

#[derive(Error, Debug)]
pub enum ImageLoadError {
    #[error("Unsupported image format")]
    UnsupportedFormat(gltf::image::Format),
    #[error("Failed to convert data")]
    DataConversionFailed(bytemuck::PodCastError),
}

pub fn parse_image(data: gltf::image::Data) -> Result<GpuImage, ImageLoadError> {
    let bytes = match data.format {
        // Add 4 byte alignment to formats which have 3 channels
        Format::R8G8B8 => data
            .pixels
            .chunks_exact(3)
            .map(|pixel| [pixel[0], pixel[1], pixel[2], u8::MAX])
            .flatten()
            .collect(),
        Format::R16G16B16 => {
            let halfs = match bytemuck::try_cast_slice::<u8, u16>(&data.pixels) {
                Ok(bytes) => bytes,
                Err(err) => return Err(ImageLoadError::DataConversionFailed(err)),
            };

            let padded = halfs
                .chunks_exact(3)
                .map(|pixel| [pixel[0], pixel[1], pixel[2], u16::MAX])
                .flatten()
                .collect();

            match bytemuck::try_cast_vec(padded) {
                Ok(bytes) => bytes,
                Err(err) => return Err(ImageLoadError::DataConversionFailed(err.0)),
            }
        }
        Format::R32G32B32FLOAT => {
            let floats = match bytemuck::try_cast_slice::<u8, f32>(&data.pixels) {
                Ok(bytes) => bytes,
                Err(err) => return Err(ImageLoadError::DataConversionFailed(err)),
            };

            let padded = floats
                .chunks_exact(3)
                .map(|pixel| [pixel[0], pixel[1], pixel[2], f32::MAX])
                .flatten()
                .collect();

            match bytemuck::try_cast_vec(padded) {
                Ok(bytes) => bytes,
                Err(err) => return Err(ImageLoadError::DataConversionFailed(err.0)),
            }
        }

        // Simply just returned the data for formats which are nicely paddded
        Format::R8
        | Format::R8G8
        | Format::R8G8B8A8
        | Format::R16G16B16A16
        | Format::R32G32B32A32FLOAT => data.pixels,

        // Return an error for unsupported formats
        f => {
            return Err(ImageLoadError::UnsupportedFormat(f));
        }
    };

    // Translate the GLTF format into a Vulkan format
    let vulkan_format = match data.format {
        // Standard 8-bit formats
        gltf::image::Format::R8 => vk::Format::R8_UNORM,
        gltf::image::Format::R8G8 => vk::Format::R8G8_UNORM,
        // 3 Channel formats have poor support, so we should convert to a 4 channel format
        gltf::image::Format::R8G8B8 => vk::Format::R8G8B8A8_UNORM,
        gltf::image::Format::R8G8B8A8 => vk::Format::R8G8B8A8_UNORM,

        // 16-bit formats
        gltf::image::Format::R16 => vk::Format::R16_UNORM,
        gltf::image::Format::R16G16 => vk::Format::R16G16_UNORM,
        // 3 Channel formats have poor support, so we should convert to a 4 channel format
        gltf::image::Format::R16G16B16 => vk::Format::R16G16B16A16_UNORM,
        gltf::image::Format::R16G16B16A16 => vk::Format::R16G16B16A16_UNORM,

        // 32-Bit Floating point formats
        // 3 Channel formats have poor support, so we should convert to a 4 channel format
        gltf::image::Format::R32G32B32FLOAT => vk::Format::R32G32B32A32_SFLOAT,
        gltf::image::Format::R32G32B32A32FLOAT => vk::Format::R32G32B32A32_SFLOAT,
    };

    let dims = glam::UVec3 {
        x: data.width,
        y: data.height,
        z: 1,
    };

    Ok(GpuImage {
        bytes,
        dims,
        format: vulkan_format,
    })
}
