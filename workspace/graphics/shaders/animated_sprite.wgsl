struct GlobalUniforms {
    view_proj: mat4x4<f32>,
    cam_pos: vec3<f32>,
    elapsed_time: f32
}
@group(0) @binding(0)
var<uniform> globals: GlobalUniforms;

@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var u_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct InstanceInput {
    @location(6) model_col_0: vec4<f32>,
    @location(7) model_col_1: vec4<f32>,
    @location(8) model_col_2: vec4<f32>,
    @location(9) model_col_3: vec4<f32>,
    @location(10) tint: vec4<f32>,
    @location(11) bounds: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var model_matrix = mat4x4<f32>(
        instance.model_col_0,
        instance.model_col_1,
        instance.model_col_2,
        instance.model_col_3,
    );

    var out: VertexOutput;
    out.clip_position = globals.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.tex_coords = (model.uv * instance.bounds.zw) + instance.bounds.xy;
    out.tint = instance.tint;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample_color = textureSample(diffuse, u_sampler, in.tex_coords);

    return sample_color * in.tint;
}
