# Shader Node Editor

Shader graphs are an **optional alternative to editing materials by hand**. One graph attaches per object and overrides the object's material surface channels — Base Color, Metallic, Roughness, Emissive, Alpha, and Normal — by wiring small typed nodes together. Graphs compile to WGSL and recompile live in the editor viewport, player, and server without any runtime code loading.

```sh
cargo run -p bozzard-editor-app -- --scene examples/demo/scenes/shader-node-lab.json
```

The **Pulse Cube** pulses its emissive glow: Time → Sine → Make Vector → Scale Vector → Master Emissive. The **Fade Cube** drives Base Color and Alpha from its Base Color texture: Texture Sample → Master Base Color, and Texture Sample Alpha → One Minus → Master Alpha, cutting the checker pattern out of the cube.

## Build without code

1. Select an object with a Mesh Renderer in Hierarchy. Choose **Properties → SHADER GRAPH → + New** (or **Load…**). One graph attaches per object, like a Material in other engines.
2. Open the **Shader** workspace tab (**View → Shader Editor** also switches). Use **+ Add node** and search by name. Drag node headers to position them; select and **Delete node** (or Delete) to remove a node and its wires. The Master node cannot be deleted — every graph has exactly one.
3. Click an output pin, then an input pin (drag/release also works). Green pins carry one float, blue pins carry a 3-component vector (linear RGB or 3D). Only matching types connect, and one wire feeds each input; a new connection replaces the input's existing wire. Right-click an input to disconnect. Escape cancels a pending connection/selection.
4. Edit unconnected input values directly on nodes. Color nodes show a color picker; other constants show drag fields. Texture Sample nodes choose one of the object's five material map slots.
5. Middle/right-drag or scroll pans; Ctrl+scroll/pinch zooms; **Fit graph** frames all nodes.
6. Check the **Scene** tab for a live preview — the graph recompiles on every edit. Editing is disabled during Play. Ctrl/Cmd+Z and Redo use normal scene history.

## Channels, defaults, and master semantics

A graph **overrides only the Master inputs you connect**. Unconnected channels keep the object's stock behavior: metallic and roughness fall back to the Drawable/Material values, Alpha to the texture's alpha, and so on. Master values clamp where the renderer requires: Metallic `0..1`, Roughness `0.045..1`, Alpha `0..1`; Base Color and Emissive stay raw so HDR emissive is possible. Alpha below the object's existing discard threshold still cuts the fragment out, in the same place as stock rendering.

The **Normal** pin is pre-TBN: on imported models it is tangent-space (default: the mapped normal map), on procedural cubes/quads it is world-space (default: the geometric normal). The host normalizes it.

Graph objects are excluded from static GI bakes, like blueprint owners: their lighting contribution is not baked, since the surface can change.

## Textures and parameters

Texture Sample reads one of the drawable's existing five map slots — Base Color, Normal, Metallic Roughness, Occlusion, or Emissive — with the same samplers and fallbacks as stock materials. Metallic Roughness follows the glTF convention: metallic is blue, roughness is green. All maps sample the Base Color UV set; per-map UV offsets from imported models are not exposed to graphs yet.

Input nodes cover Time (simulation seconds: it advances only in Play, and the shader preview pane runs its own clock; Effects Live preview animates particles and atmosphere but keeps Time at zero), UV, World Normal, World Position, and View Direction. Constant nodes cover Float, Color, and Vector. Float math: Add, Subtract, Multiply, Divide (zero-safe), Power, Sine, Clamp, Lerp, One Minus. Vector math: Add Vectors, Multiply Vectors, Scale Vector, Lerp Vectors, Dot Product, Normalize, Make Vector, Split Vector. Limits match Blueprints: 128 nodes, 512 wires, finite constants, acyclic graphs — validation rejects anything else on load, save, and connect.

## Save, reuse, and attach copies

- The Content Browser's **Shader graphs** folder lists saved graphs under scene-relative `assets/ShaderGraphs/` (and `assets/*.shadergraph.json`) plus attached graphs. Double-click a saved file to attach a copy to the selected object; clicking an attached-graph entry selects its owner.
- **Save graph…** (workspace tab or Properties) exports to a `.shadergraph.json` file, defaulting to `assets/ShaderGraphs/`. Existing files require explicit replacement confirmation; writes use atomic replacement. Scene Undo does not undo file exports.
- **Load copy…** or Properties **Load…** attaches an independent copy, replacing the object's current graph. This is not a live file link: later edits to the file cannot silently change an object. A scene embeds the whole graph — node positions, constants, wires — so Save/Open need no extra files at runtime.
- Object duplication and prefab capture carry the graph along as ordinary object data.

## Pipeline behavior

Each graph compiles into one WGSL override function spliced into the scene shader hosts, and the renderer caches one pipeline set per graph (opaque and transparent, basic and PBR flavors) keyed by a content hash of the generated code. Editing a graph recompiles only that graph's pipelines; stock rendering is bit-identical when no channel is connected. Graph objects share the normal draw pipeline selection, sorting, shadows, fog, and post-processing; the graph replaces the surface channels, not the lighting model. Unlit materials evaluate Emissive additively after their flat color, as with stock unlit shading.

Current limits: no custom samplers, tiling/offset controls, UV sets, vertex-displacement, or instanced properties; texturing uses only the five existing slots; procedural meshes bind neutral placeholders for the four non-color maps. Compile errors surface as an editor status message rather than a crash — graphs are validated before codegen, so a broken pipeline is a bug, not a scene problem. See [Gameplay Blueprints](blueprints.md) for the same node-pane interaction model applied to behavior, and [Scene documents](scenes.md) for the object schema the graph embeds into.
