struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};


@group(0) @binding(0) var<storage, read> projection_matrices: array<mat4x4<f32>, 1024>;

var<push_constant> matrix_index: u32;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = projection_matrices[matrix_index] * vec4<f32>(model.position, 1.0);
    out.color = model.normal * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}