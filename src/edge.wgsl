struct Camera {
    view_proj: mat4x4<f32>,
    dimensions: vec2<f32>,
    scale: f32,
};

@binding(0) @group(0) var<uniform> camera: Camera;

struct VertexInput {
    @location(0) pos: vec3<f32>,
};

struct Edge {
    @location(1) color: vec4<f32>,
    @location(2) a: vec3<f32>,
    @location(3) b: vec3<f32>,
    @location(4) a_norm: vec3<f32>,
    @location(5) b_norm: vec3<f32>,
    @location(6) line_width: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput, edge: Edge, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos: vec3<f32>;
    var norm: vec3<f32>;

    switch vertex_index {
        case 0u {
            pos = edge.a;
            norm = edge.a_norm;
        }
        case 1u {
            pos = edge.a;
            norm = edge.a_norm * -1.0;
        }
        case 2u {
            pos = edge.b;
            norm = edge.b_norm;
        }
        case 3u {
            pos = edge.b;
            norm = edge.b_norm * -1.0;
        }
        case 4u {
            pos = edge.b;
            norm = edge.b_norm;
        }
        default: {
            pos = edge.a;
            norm = edge.a_norm;
        }
    }

    // let line_width = vec3<f32>(1.0, 600.0/800.0, 1.0);
    // let norm = camera.view_proj * vec4<f32>(norm.xy, 0.0, 0.0);

    let delta = vec4<f32>(norm.xyz * edge.line_width, 1.0);
    // let delta = vec4<f32>(norm.xyz * line_width, 1.0);

    // let delta = vec4<f32>(delta.x, .y * (600.0/800.0), norm.z, norm.w);

    let pos = camera.view_proj * vec4<f32>(pos.xy + delta.xy, 0.0, 1.0);
    // let pos = vec4<f32>(pos.xy + delta.xy, 0.1, 1.0);
    // let pos = vec4<f32>(pos.xy, max(edge.a.z, edge.b.z), 1.0);
    let pos = vec4<f32>(pos.xy, 0.1, 1.0);

    var out: VertexOutput;
    out.position = pos;
    out.color = edge.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color.rgba);
}

