// Vertex shader

struct GlobalUniforms {
    view_proj: mat4x4<f32>,
    cam_pos: vec3<f32>,
    elapsed_time: f32
}
@group(0) @binding(0)
var<uniform> globals: GlobalUniforms;

@group(1) @binding(0)
var<uniform> color: vec4<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) model_col_0: vec4<f32>,
    @location(2) model_col_1: vec4<f32>,
    @location(3) model_col_2: vec4<f32>,
    @location(4) model_col_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(instance: VertexInput) -> VertexOutput {
    var model_matrix = mat4x4<f32>(
        instance.model_col_0,
        instance.model_col_1,
        instance.model_col_2,
        instance.model_col_3,
    );

    var out: VertexOutput;
    out.clip_position = globals.view_proj * model_matrix * vec4<f32>(instance.position, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return color;
}