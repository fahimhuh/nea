use self::{scene::Scene, shaders::Uniforms};

use super::frame::FrameRef;
use crate::{
    loader::SceneLoader,
    vulkan::{
        command::CommandList,
        context::Context,
        descriptor::{
            DescriptorBinding, DescriptorBufferWrite, DescriptorImageWrite, DescriptorPool,
            DescriptorSet, DescriptorSetLayout, DescriptorTLASWrite,
        },
        image::Image,
        pipeline::{ComputePipeline, PipelineLayout},
        shader::Shader,
    },
    world::World,
};
use ash::vk::{self};

use std::sync::Arc;

mod scene;
mod shaders;
mod shader {
    include!(concat!(env!("OUT_DIR"), "/raytracer.comp.rs"));
}

pub struct Raytracer {
    descriptor_pool: DescriptorPool,
    descriptor_layout: DescriptorSetLayout,
    pipeline_layout: PipelineLayout,
    shader: Shader,
    pipeline: ComputePipeline,
    descriptor_sets: Vec<DescriptorSet>,

    uniforms: Uniforms,
    scene: Option<Scene>,
}

impl Raytracer {
    pub fn new(context: Arc<Context>) -> Self {
        let descriptor_pool = DescriptorPool::new(context.clone());

        let bindings = vec![
            DescriptorBinding {
                binding: 0,
                count: 1,
                kind: vk::DescriptorType::STORAGE_IMAGE,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorBinding {
                binding: 1,
                count: 1,
                kind: vk::DescriptorType::UNIFORM_BUFFER,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorBinding {
                binding: 2,
                count: 1,
                kind: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
            DescriptorBinding {
                binding: 3,
                count: 1,
                kind: vk::DescriptorType::STORAGE_BUFFER,
                stage: vk::ShaderStageFlags::COMPUTE,
            },
        ];

        let descriptor_layout = DescriptorSetLayout::new(context.clone(), bindings);
        let push_constants = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<u64>() as u32,
        };

        let pipeline_layout = PipelineLayout::new(
            context.clone(),
            push_constants,
            std::slice::from_ref(&descriptor_layout),
        );

        let shader = Shader::new(
            context.clone(),
            &shader::CODE,
            vk::ShaderStageFlags::COMPUTE,
            "main",
        );

        let pipeline = ComputePipeline::new(context.clone(), &shader, &pipeline_layout);
        let descriptor_sets = descriptor_pool.allocate(&context, &descriptor_layout, 3);

        let uniforms = Uniforms::new(context.clone());

        Self {
            descriptor_pool,
            descriptor_layout,
            pipeline_layout,
            shader,
            pipeline,
            descriptor_sets,
            uniforms,
            scene: None,
        }
    }

    pub fn run(&mut self, cmds: &CommandList, frame: &FrameRef, world: &World) {
        if let Some(scene) = SceneLoader::poll() {
            self.scene = Some(Scene::load(frame.context.clone(), scene));
        }

        // Only raytrace if there is a scene to trace against!
        if self.scene.is_some() {
            self.raytrace(cmds, frame, world);
        }
    }

    fn raytrace(&mut self, cmds: &CommandList, frame: &FrameRef, world: &World) {
        let scene = self.scene.as_ref().unwrap();
        let uniforms = self.uniforms.update_uniforms(frame, world);

        let descriptor_set = self.descriptor_sets.get(frame.index()).unwrap();
        descriptor_set.write(
            &[DescriptorImageWrite {
                image_view: frame.display.views.get(frame.index()).unwrap(),
                layout: vk::ImageLayout::GENERAL,
                binding: 0,
                sampler: None,
                image_kind: vk::DescriptorType::STORAGE_IMAGE,
            }],
            &[
                DescriptorBufferWrite {
                    buffer_kind: vk::DescriptorType::UNIFORM_BUFFER,
                    buffer: uniforms,
                    range: Uniforms::UNIFORMS_SIZE,
                    binding: 1,
                },
                DescriptorBufferWrite {
                    buffer_kind: vk::DescriptorType::STORAGE_BUFFER,
                    buffer: &scene.materials,
                    range: Scene::MATERIAL_BUFFER_SIZE,
                    binding: 3,
                },
            ],
        );

        descriptor_set.write_tlas(DescriptorTLASWrite {
            reference: &scene.tlas,
            binding: 2,
        });

        let image_memory_barriers = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::NONE,
            src_access_mask: vk::AccessFlags2::NONE,
            dst_stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
            dst_access_mask: vk::AccessFlags2::SHADER_WRITE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::GENERAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: frame.display.images.get(frame.index()).unwrap().handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];

        cmds.pipeline_barrier(&image_memory_barriers, &[]);

        cmds.bind_compute_pipeline(&self.pipeline);
        cmds.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.pipeline_layout,
            &[descriptor_set.handle],
        );

        cmds.dispatch(frame.display.dims.x, frame.display.dims.y, 1);

        let image_memory_barriers = [vk::ImageMemoryBarrier2 {
            src_stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
            src_access_mask: vk::AccessFlags2::SHADER_WRITE,
            dst_stage_mask: vk::PipelineStageFlags2::NONE,
            dst_access_mask: vk::AccessFlags2::NONE,
            old_layout: vk::ImageLayout::GENERAL,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: frame.display.images.get(frame.index()).unwrap().handle,
            subresource_range: Image::default_subresource(vk::ImageAspectFlags::COLOR),
            ..Default::default()
        }];
        cmds.pipeline_barrier(&image_memory_barriers, &[]);
    }
}
