struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct PointLight {
    position: vec4<f32>,
    color_and_intensity: vec4<f32>
};


@group(0) @binding(0) var<storage, read> mvp_matrices: array<mat4x4<f32>, 1024>;
@group(0) @binding(1) var<storage, read> model_matrices: array<mat4x4<f32>, 1024>;
@group(0) @binding(2) var<storage, read> normal_matrices: array<mat4x4<f32>, 1024>;
@group(0) @binding(3) var<storage, read> point_lights: array<PointLight, 64>;

var<push_constant> matrix_index: u32;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mvp_matrices[matrix_index] * vec4<f32>(model.position, 1.0);
    out.world_position = (model_matrices[matrix_index] * vec4<f32>(model.position, 1.0)).xyz;
    out.normal = (normal_matrices[matrix_index] * vec4<f32>(model.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    var final_color = vec3(0.0);

    for (var i = 0; i < 64; i += 1)
    {
        let light = point_lights[i]; 
    
        let light_color = light.color_and_intensity.xyz;
        let light_intensity = light.color_and_intensity.w;
        let light_pos = light.position.xyz;
        let to_light_vector = light_pos - in.world_position;
        let d = length(to_light_vector);
        let attenuation = 1.0 / (d * d);
        
        let ambient_strength = 0.001;
        let ambient_color = light_color * ambient_strength;

        let normal = normalize(in.normal);
        let light_dir = normalize(to_light_vector);
        let diffuse_factor = max(dot(normal, light_dir), 0.0);
        let diffuse_color = light_color * light_intensity * diffuse_factor * attenuation;
        final_color += ambient_color + diffuse_color;
    }
   
    return vec4<f32>(final_color, 1.0);
}