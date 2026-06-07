struct GlobalUniforms {
    view_proj: mat4x4<f32>,
    cam_pos: vec3<f32>,
    elapsed_time: f32
}
@group(0) @binding(0)
var<uniform> globals: GlobalUniforms;

@group(1) @binding(0) var font_atlas: texture_2d<f32>;
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
    @location(10) text_color: vec4<f32>,
    @location(11) outline_color: vec4<f32>,
    @location(12) bounds: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) t_color: vec4<f32>,
    @location(2) o_color: vec4<f32>,
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
    // out.tex_coords = model.uv;
    out.t_color = instance.text_color;
    out.o_color = instance.outline_color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = textureSample(font_atlas, u_sampler, in.tex_coords).r;
    let change = fwidth(distance);
    let text_edge = 0.5;
    let outline_edge = 0.4;

    let text_alpha = smoothstep(text_edge - change, text_edge + change, distance);
    let outline_alpha = smoothstep(outline_edge - change, outline_edge + change, distance) * in.o_color.a;

    let text_color = in.t_color.rbg;
    let outline_color = in.o_color.rbg;

    let color = mix(outline_color, text_color, text_alpha);
    let alpha = max(in.t_color.a * text_alpha, outline_alpha);

    return vec4<f32>(color, alpha);
}