//! Text overlay composited after display effects, with no world depth or camera transform.
use super::*;

pub(super) struct HudRenderer {
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    encode: bool,
}
impl HudRenderer {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("HUD text"),
                source: wgpu::ShaderSource::Wgsl(include_str!("hud.wgsl").into()),
            });
        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HUD overlay"), layout: None,
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 32, step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        });
        Self {
            pipeline,
            sampler: gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("HUD glyph sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            encode: !format.is_srgb(),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
        scene: &RenderScene,
        text: &text::TextRenderer,
        raw: bool,
        scale: f32,
    ) -> Result<()> {
        let Some(atlas) = &text.view else {
            return Ok(());
        };
        let layout = self.pipeline.get_bind_group_layout(0);
        let mut draws = Vec::new();
        for item in &scene.items {
            let MeshKind::Text(settings) = &item.mesh else {
                continue;
            };
            let Some(screen) = settings.screen else {
                continue;
            };
            let Some(mesh) = text.mesh(settings) else {
                continue;
            };
            ensure!(
                item.material
                    .tint
                    .iter()
                    .all(|c| c.is_finite() && (0.0..=1.0).contains(c)),
                "invalid HUD tint"
            );
            let buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("HUD text uniform"),
                    contents: &float_bytes(
                        screen
                            .matrix(size, scale)
                            .to_cols_array()
                            .into_iter()
                            .chain(item.material.tint)
                            .chain([
                                settings.opacity,
                                if self.encode && !raw { 1. } else { 0. },
                                0.,
                                0.,
                                0.,
                            ]),
                    ),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let binding = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("HUD glyph binding"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(atlas),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            draws.push((mesh, binding));
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HUD after display"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        for (mesh, binding) in &draws {
            pass.set_bind_group(0, binding, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(mesh.vertex_offset..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.count, 0, 0..1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hud_pixels_ignore_camera_and_effects_follow_resize_and_release_resources() -> Result<()> {
        let instance = crate::instance(crate::Backend::native());
        let gpu = pollster::block_on(Gpu::request(
            &instance,
            None,
            cfg!(not(target_os = "macos")),
        ))?;
        let mut renderer = SceneRenderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm);
        let mut scene = RenderScene {
            particles: vec![],
            fog: Default::default(),
            gi: None,
            lights: vec![],
            environment: Default::default(),
            display: Default::default(),
            lighting: Default::default(),
            view_projection: Mat4::IDENTITY,
            items: vec![DrawItem {
                motion_id: 0,
                model: Mat4::from_translation(Vec3::splat(1000.)),
                mesh: MeshKind::Text(TextMesh {
                    text: "HUD 42".into(),
                    font_size: 28.,
                    screen: Some(ScreenText {
                        anchor: [1., 0.],
                        offset: [-20., 20.],
                    }),
                    alignment: TextAlignment::Right,
                    ..Default::default()
                }),
                material: Material {
                    metallic: None,
                    roughness: None,
                    surface_overrides: Default::default(),
                    tint: [0., 1., 0.],
                    uv_scale: [1.; 2],
                    texture: TextureKind::Text,
                    lit: false,
                },
            }],
        };
        let mut occluder = scene.items[0].clone();
        occluder.mesh = MeshKind::Quad;
        occluder.material.texture = TextureKind::White;
        occluder.material.tint = [0.1; 3];
        occluder.model = Mat4::from_scale_rotation_translation(
            Vec3::new(3., 3., 1.),
            glam::Quat::IDENTITY,
            Vec3::new(0., 0., 0.1),
        );
        scene.items.push(occluder);
        let capture = |renderer: &mut SceneRenderer, scene: &RenderScene, w, h| {
            crate::capture_offscreen(&gpu, w, h, |target| {
                renderer.draw(&gpu, target, [w, h], scene)
            })
        };
        let coordinates = |image: &crate::Frame| -> Vec<[usize; 2]> {
            image
                .rgba
                .chunks_exact(4)
                .enumerate()
                .filter(|(_, p)| p[1] > 180 && p[0] < 20 && p[2] < 20)
                .map(|(i, _)| [i % image.width as usize, i / image.width as usize])
                .collect()
        };
        let a = capture(&mut renderer, &scene, 320, 240)?;
        let original = coordinates(&a);
        assert!(original.len() > 80);
        assert!(
            original
                .iter()
                .all(|p| p[0] > 180 && p[0] < 301 && p[1] >= 20 && p[1] < 65)
        );
        scene.view_projection = Mat4::from_translation(Vec3::splat(4.));
        scene.display.exposure_ev = -8.;
        scene.display.vignette.intensity = 1.;
        let same_glyphs = |actual: Vec<[usize; 2]>, expected: Vec<[usize; 2]>| {
            let actual: BTreeSet<_> = actual.into_iter().collect();
            let expected: BTreeSet<_> = expected.into_iter().collect();
            // Antialiased edges blend with the world behind them; changed background
            // brightness can move the color threshold by a few edge pixels.
            assert!(actual.intersection(&expected).count() * 100 >= expected.len() * 95);
            assert!(actual.symmetric_difference(&expected).count() * 100 <= expected.len() * 10);
        };
        same_glyphs(
            coordinates(&capture(&mut renderer, &scene, 320, 240)?),
            original.clone(),
        );
        let resized = coordinates(&capture(&mut renderer, &scene, 640, 360)?);
        same_glyphs(
            resized,
            original.iter().map(|p| [p[0] + 320, p[1]]).collect(),
        );
        if let MeshKind::Text(text) = &mut scene.items[0].mesh {
            text.text = "HUD 99".into();
        }
        assert_ne!(
            coordinates(&capture(&mut renderer, &scene, 320, 240)?),
            original
        );
        scene.items.clear();
        assert!(coordinates(&capture(&mut renderer, &scene, 320, 240)?).is_empty());
        assert!(renderer.text.is_none() && renderer.hud.is_none());
        println!("hud_gpu_ok anchoring resize camera_effect_isolation dynamic_text cleanup");
        Ok(())
    }
}
