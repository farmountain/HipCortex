use anyhow::Result;
use image::{DynamicImage, GenericImageView};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::path::Path;

pub struct VisionEncoder;

impl VisionEncoder {
    /// Encode a DynamicImage into a simple RGB mean vector.
    pub fn encode_image(image: &DynamicImage) -> Vec<f32> {
        let (w, h) = image.dimensions();
        let rgb = image.to_rgb8();
        let mut r_sum = 0u64;
        let mut g_sum = 0u64;
        let mut b_sum = 0u64;
        for p in rgb.pixels() {
            r_sum += p[0] as u64;
            g_sum += p[1] as u64;
            b_sum += p[2] as u64;
        }
        let total = (w * h) as f32 * 255.0;
        vec![
            r_sum as f32 / total,
            g_sum as f32 / total,
            b_sum as f32 / total,
        ]
    }

    #[cfg(feature = "parallel")]
    pub fn encode_images_parallel(images: &[DynamicImage]) -> Vec<Vec<f32>> {
        images.par_iter().map(Self::encode_image).collect()
    }

    #[cfg(feature = "gpu")]
    pub async fn encode_image_gpu(bytes: &[u8]) -> Result<Vec<f32>> {
        use wgpu::util::DeviceExt;
        let instance = wgpu::Instance::default();

        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
        {
            Some(a) => a,
            None => {
                let img = image::load_from_memory(bytes)?;
                return Ok(Self::encode_image(&img));
            }
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;
        let img = image::load_from_memory(bytes)?;
        let data = img.to_rgb8();
        let len = (data.width() * data.height() * 3) as u64;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &data,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 12,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/avg.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(len as u32 / 3, 1, 1);
        }
        queue.submit(Some(encoder.finish()));
        let buffer_slice = output.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });

        device.poll(wgpu::Maintain::Wait);
        rx.await.unwrap()?;
        let data = buffer_slice.get_mapped_range();
        let vec = bytemuck::cast_slice::<u8, f32>(&data).to_vec();
        drop(data);
        output.unmap();
        Ok(vec)
    }

    /// Encode raw image bytes (PNG/JPEG) into an embedding.
    pub fn encode_bytes(bytes: &[u8]) -> Result<Vec<f32>> {
        let img = image::load_from_memory(bytes)?;
        Ok(Self::encode_image(&img))
    }

    /// Encode image from a file path.
    pub fn encode_path<P: AsRef<Path>>(path: P) -> Result<Vec<f32>> {
        let img = image::open(path)?;
        Ok(Self::encode_image(&img))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};

    #[test]
    fn encode_image_basic() {
        let img = RgbImage::from_pixel(1, 1, image::Rgb([0, 0, 255]));
        let dynimg = DynamicImage::ImageRgb8(img);
        let emb = VisionEncoder::encode_image(&dynimg);
        assert_eq!(emb.len(), 3);
    }
}
