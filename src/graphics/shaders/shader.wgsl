// Vertex shader

struct GlobalUniforms {
    view_proj: mat4x4<f32>,
    cam_pos: vec3<f32>,
    elapsed_time: f32
}
@group(0) @binding(0)
var<uniform> globals: GlobalUniforms;

struct Material {
    model_matrix: mat4x4<f32>,
    color: vec4<f32>,
}
@group(1) @binding(0)
var<uniform> material: Material;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // var x = model.position.x + cos(globals.elapsed_time*2.0)*0.2;
    // var y = model.position.y + sin(globals.elapsed_time*2.0)*0.2;

    // var pos = vec4<f32>(x, y, model.position.z, 1.0);
    // out.clip_position = globals.view_proj * material.model_matrix * pos;

    out.clip_position = globals.view_proj * material.model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return material.color;
}