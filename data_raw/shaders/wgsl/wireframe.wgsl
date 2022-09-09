#import "utils.wgsl"
#import "common.wgsl"

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct FragmentOutput {
    @location(0) albedo: vec4<f32>,
};


@group(0) @binding(0)
var<uniform> constant_data: ConstantData;
@group(0) @binding(1)
var<storage, read> positions: Positions;
@group(0) @binding(2)
var<storage, read> colors: Colors;
@group(0) @binding(3)
var<storage, read> meshes: Meshes;
@group(0) @binding(4)
var<storage, read> meshes_aabb: AABBs;

@vertex
fn vs_main(
    v_in: Vertex,
) -> VertexOutput {
    let mesh = &meshes.data[v_in.mesh_index];
    let aabb = &meshes_aabb.data[v_in.mesh_index];
    
    let aabb_size = abs((*aabb).max - (*aabb).min);
    let position = (*aabb).min + decode_as_vec3(positions.data[v_in.position_and_color_offset]) * aabb_size;

    let mvp = constant_data.proj * constant_data.view;
    var vertex_out: VertexOutput;
    vertex_out.clip_position = mvp * vec4<f32>(transform_vector(position, (*mesh).position, (*mesh).orientation, (*mesh).scale), 1.0);

    vertex_out.color = unpack_unorm_to_4_f32(colors.data[v_in.position_and_color_offset]);

    return vertex_out;
}

@fragment
fn fs_main(
    v_in: VertexOutput,
) -> FragmentOutput {    
    var fragment_out: FragmentOutput;
    fragment_out.albedo = v_in.color;
    return fragment_out;
}