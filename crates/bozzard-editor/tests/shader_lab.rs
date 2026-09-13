use bozzard_render_assets::shader_source;
use bozzard_scene::Scene;

#[test]
fn shader_lab_loads_and_codegens_all_graphs() {
    let scene = Scene::from_json(include_str!(
        "../../../examples/demo/scenes/shader-node-lab.json"
    ))
    .unwrap();
    let graphs: Vec<_> = scene
        .objects
        .iter()
        .filter_map(|o| o.shader_graph.as_ref())
        .collect();
    let names: Vec<_> = graphs.iter().map(|g| g.name.as_str()).collect();
    assert!(
        names.contains(&"Pulse Emissive") && names.contains(&"Texture Fade"),
        "example scene should embed the two example graphs, found {names:?}"
    );
    for graph in &graphs {
        graph.validate().unwrap();
        let code = shader_source(graph).unwrap();
        assert!(code.surface.contains("graph_material_surface"));
        assert!(code.surface.contains("params."));
    }
    // Pulse Emissive drives only Emissive; Texture Fade drives Base Color and Alpha.
    let pulse = shader_source(graphs.iter().find(|g| g.name == "Pulse Emissive").unwrap())
        .unwrap()
        .surface
        .clone();
    assert!(pulse.contains("params.emissive"));
    assert!(!pulse.contains("params.base ="));
    let fade = shader_source(graphs.iter().find(|g| g.name == "Texture Fade").unwrap())
        .unwrap()
        .surface
        .clone();
    assert!(fade.contains("params.base ="));
    assert!(fade.contains("params.alpha = clamp"));
}
