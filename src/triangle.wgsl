struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.5),         
        vec2<f32>(-0.5, -0.5),      
        vec2<f32>(0.5, -0.5)   
    );

    let colors = array<vec4<f32>, 3>(
        vec4<f32>(1.0, 0.0, 0.0, 1.0),
        vec4<f32>(0.0, 1.0, 0.0, 1.0),
        vec4<f32>(0.0, 0.0, 1.0, 1.0)  
    );

    let pos = positions[in_vertex_index];
    let color = colors[in_vertex_index];


    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos.x, pos.y, 0.0, 1.0); // x y z w
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}