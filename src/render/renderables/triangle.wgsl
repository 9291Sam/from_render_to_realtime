

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> projection_matrices: array<mat4x4<f32>, 1024>;

var<push_constant> matrix_index: u32;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    let positions = array<vec3<f32>, 3>(
        vec3<f32>(0.0, 0.5, 0.0),         
        vec3<f32>(-0.5, -0.5, 0.0),      
        vec3<f32>(0.5, -0.5, 0.0)   
    );

    let colors = array<vec4<f32>, 3>(
        vec4<f32>(1.0, 0.0, 0.0, 1.0),
        vec4<f32>(0.0, 1.0, 0.0, 1.0),
        vec4<f32>(0.0, 0.0, 1.0, 1.0)  
    );

    let pos = positions[in_vertex_index];
    let color = colors[in_vertex_index];


    var out: VertexOutput;
    out.clip_position = projection_matrices[matrix_index] * vec4<f32>(pos, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}