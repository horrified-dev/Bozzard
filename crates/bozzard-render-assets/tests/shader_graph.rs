//! Shader graph modules: codegen output compiles and overrides surface channels
//! through the same lighting pipeline as stock materials.
use bozzard_render::*;
use bozzard_scene::shader_graph::{Node, NodeKind, ShaderGraph, Value};
use glam::{Mat4, Vec3};

fn graph_surface(graph: ShaderGraph) -> std::sync::Arc<ShaderSource> {
    bozzard_render_assets::shader_source(&graph).unwrap()
}

fn item(shader: Option<std::sync::Arc<ShaderSource>>) -> DrawItem {
    DrawItem {
        motion_id: 0,
        mesh: MeshKind::Quad,
        model: Mat4::from_translation(Vec3::new(0., 0., -5.))
            * Mat4::from_scale(Vec3::new(4., 3., 1.)),
        material: Material {
            metallic: None,
            roughness: None,
            tint: [1., 1., 1.],
            lit: false,
            texture: TextureKind::White,
            uv_scale: [1.; 2],
            surface_overrides: Default::default(),
            shader,
        },
    }
}

fn scene(items: Vec<DrawItem>) -> RenderScene {
    RenderScene {
        shader_time: 0.,
        particles: Vec::new(),
        view_projection: glam::camera::rh::proj::directx::orthographic(-4., 4., -3., 3., 0.1, 30.),
        items,
        lighting: Lighting {
            shadows: false,
            sun_intensity: 0.,
            ambient_intensity: 0.,
            ..Default::default()
        },
        lights: vec![],
        environment: EnvironmentSettings::disabled(),
        fog: Default::default(),
        gi: None,
        display: DisplaySettings {
            tone_mapping: false,
            ..Default::default()
        },
    }
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * frame.width + x) * 4) as usize;
    frame.rgba[i..i + 3].try_into().unwrap()
}

fn render(items: Vec<DrawItem>, size: [u32; 2]) -> anyhow::Result<Frame> {
    let gpu = pollster::block_on(Gpu::request(&instance(Backend::native()), None, false))?;
    let mut renderer = SceneRenderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm);
    let scene = scene(items);
    capture_offscreen(&gpu, size[0], size[1], |target| {
        renderer.draw_linear(&gpu, target, size, &scene)
    })
}

#[test]
fn base_color_graph_replaces_material_and_tint() -> anyhow::Result<()> {
    let mut graph = ShaderGraph::default();
    let color = Node::new(2, NodeKind::Color, [0., 0.]);
    graph.nodes.push(color);
    graph
        .connect(bozzard_scene::shader_graph::Wire {
            from: bozzard_scene::shader_graph::Socket { node: 2, port: 0 },
            to: bozzard_scene::shader_graph::Socket { node: 1, port: 0 },
        })
        .unwrap();
    graph.nodes[1].inputs[0] = Value::Vector([0.9, 0.1, 0.1]);
    // The tint would turn the quad cyan without the graph override.
    let mut item = item(Some(graph_surface(graph)));
    item.material.tint = [0., 1., 1.];
    let frame = render(vec![item], [32, 24])?;
    let center = pixel(&frame, 16, 12);
    // Linear RGB through draw_linear: 0.9 -> 229, 0.1 -> 25.
    assert!(
        (226..=232).contains(&center[0])
            && (22..=28).contains(&center[1])
            && (22..=28).contains(&center[2]),
        "expected red quad, got {center:?}"
    );
    Ok(())
}

#[test]
fn alpha_graph_discards_pixels() -> anyhow::Result<()> {
    let mut graph = ShaderGraph::default();
    graph.nodes.push(Node::new(2, NodeKind::Float, [0., 0.]));
    graph
        .connect(bozzard_scene::shader_graph::Wire {
            from: bozzard_scene::shader_graph::Socket { node: 2, port: 0 },
            to: bozzard_scene::shader_graph::Socket { node: 1, port: 4 },
        })
        .unwrap();
    // Alpha 0 discards the whole quad; the clear color shows through.
    let frame = render(vec![item(Some(graph_surface(graph)))], [32, 24])?;
    let center = pixel(&frame, 16, 12);
    assert_eq!(
        center,
        [5, 6, 10],
        "expected clear background, got {center:?}"
    );
    Ok(())
}

#[test]
fn sphere_preview_renders_graph_geometry() {
    // Preview geometry: a base-color graph on the UV sphere, unlit for exact colors.
    let mut graph = ShaderGraph::default();
    graph.nodes.push(Node::new(2, NodeKind::Color, [0., 0.]));
    graph.nodes[1].inputs[0] = Value::Vector([0.9, 0.1, 0.1]);
    graph
        .connect(bozzard_scene::shader_graph::Wire {
            from: bozzard_scene::shader_graph::Socket { node: 2, port: 0 },
            to: bozzard_scene::shader_graph::Socket { node: 1, port: 0 },
        })
        .unwrap();
    let mut item = item(Some(graph_surface(graph)));
    item.mesh = bozzard_render::MeshKind::Sphere;
    let frame = render(vec![item], [48, 36]).expect("sphere render");
    let center = pixel(&frame, 24, 18);
    assert!(
        (226..=232).contains(&center[0])
            && (22..=28).contains(&center[1])
            && (22..=28).contains(&center[2]),
        "expected red sphere at center, got {center:?}"
    );
    // Corner stays background: the sphere does not fill the frame.
    let corner = pixel(&frame, 2, 2);
    assert_eq!(
        corner,
        [5, 6, 10],
        "expected background at corner, got {corner:?}"
    );
}
