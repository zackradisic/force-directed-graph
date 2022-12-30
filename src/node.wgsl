struct Camera {
    view_proj: mat4x4<f32>,
    dimensions: vec2<f32>,
    scale: f32,
};

@binding(0) @group(0) var<uniform> camera: Camera;

struct VertexInput {
    @location(0) pos: vec3<f32>
}

struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,

    @location(6) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput, instance: InstanceInput, @builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    let pos = camera.view_proj * model * vec4<f32>(in.pos, 1.0);
    // let pos = camera.view_proj * vec4<f32>(0.0, 0.0, 0.5, 1.0);
    // let pos = camera.view_proj * vec4<f32>(in.pos.xy, 0.9, 1.0);
    // let pos = camera.view_proj * vec4<f32>(in.pos.xy, 0.9, 1.0);
    out.color = instance.color;
    out.position = pos;
    // out.position = vec4<f32>(pos.xy, 0.5, pos.w);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}