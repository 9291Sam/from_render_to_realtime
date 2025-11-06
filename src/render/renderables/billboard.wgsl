fn gpu_hashU32(state_in: u32) -> u32 {
    var state = state_in;
    state = (state ^ 61u) ^ (state >> 16u);
    state = state + (state << 3u);
    state = state ^ (state >> 4u);
    state = state * 0x27d4eb2du;
    state = state ^ (state >> 15u);
    return state;
}

struct BillBoard {
    position_and_size: vec4<f32>,
    color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) is_visible: u32,
}



@group(0) @binding(0) var<storage, read> mvp_matrices: array<mat4x4<f32>, 1024>;

@group(1) @binding(0) var<storage, read> in_billboards: array<BillBoard, 65535>;

struct PushConstantData {
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    random_seed: u32,
    matrix_index: u32
}

var<push_constant> push_constant: PushConstantData;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    let IDX_TO_VTX_TABLE = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);
    
    let billboard_index = in_vertex_index / 6u;
    let point_within_face = in_vertex_index % 6u;
    let corner_index = IDX_TO_VTX_TABLE[point_within_face];

    let this_billboard = in_billboards[billboard_index];
    
    let r = push_constant.camera_right.xyz * this_billboard.position_and_size.w;
    let u = push_constant.camera_up.xyz * this_billboard.position_and_size.w;
    
    let corner_positions = array<vec3<f32>, 4>(
        this_billboard.position_and_size.xyz + -r + u,      
        this_billboard.position_and_size.xyz + -r + -u,        
        this_billboard.position_and_size.xyz + r + u,        
        this_billboard.position_and_size.xyz + r + -u   
    );

    let uvs = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), 
        vec2<f32>(0.0, 1.0), 
        vec2<f32>(1.0, 0.0), 
        vec2<f32>(1.0, 1.0)  
    );

    let mvp = mvp_matrices[push_constant.matrix_index];
    let world_pos = corner_positions[corner_index];

    // let hash = gpu_hashU32(push_constant.random_seed + billboard_index);
    // let should_be_visible = (hash % 2u) == 1u;

    var out: VertexOutput;
    out.clip_position = mvp * vec4<f32>(world_pos, 1.0);
    out.uv = uvs[corner_index];
    out.is_visible = 1u; // select(0u, 1u, should_be_visible);

    // if (should_be_visible) {
        out.color = this_billboard.color;
    // }
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let unit_position = in.uv * 2.0 - 1.0;

    if (length(unit_position) >= 1.0 || in.is_visible == 0u) {
        discard;
    }

    return in.color;
}