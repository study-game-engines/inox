
struct ConstantData {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    screen_width: f32,
    screen_height: f32,
    flags: u32,
};


struct DrawCommand {
    vertex_count: u32,
    instance_count: u32,
    base_index: u32,
    vertex_offset: i32,
    base_instance: u32,
};

struct MeshData {
    position: vec3<f32>,
    scale: f32,
    orientation: vec4<f32>,
};

struct MeshletData {
    center: vec3<f32>,
    radius: f32,
    cone_axis: vec3<f32>,
    cone_cutoff: f32,
    vertices_count: u32,
    vertices_offset: u32,
    indices_count: u32,
    indices_offset: u32,
};

struct Meshlets {
    meshlets: array<MeshletData>,
};
struct Meshes {
    meshes: array<MeshData>,
};
struct Commands {
    commands: array<DrawCommand>,
};

@group(0) @binding(0)
var<uniform> constant_data: ConstantData;
@group(0) @binding(1)
var<storage, read> meshlets: Meshlets;
@group(0) @binding(2)
var<storage, read> meshes: Meshes;
@group(0) @binding(3)
var<storage, read_write> commands: Commands;


fn rotate_quat(pos: vec3<f32>, orientation: vec4<f32>) -> vec3<f32> {
    return pos + 2.0 * cross(orientation.xyz, cross(orientation.xyz, pos) + orientation.w * pos);
}

fn cone_culling(meshlet: MeshletData, mesh: MeshData, camera_position: vec3<f32>) -> bool {
    let center = rotate_quat(meshlet.center, mesh.orientation) * mesh.scale + mesh.position;
    let radius = meshlet.radius * mesh.scale;

    let cone_axis = rotate_quat(vec3<f32>(meshlet.cone_axis[0] / 127., meshlet.cone_axis[1] / 127., meshlet.cone_axis[2] / 127.), mesh.orientation);
    let cone_cutoff = meshlet.cone_cutoff / 127.;

    let direction = center - camera_position;
    return dot(direction, cone_axis) < cone_cutoff * length(direction) + radius;
}


@compute
@workgroup_size(32, 1, 1)
fn main(@builtin(local_invocation_id) local_invocation_id: vec3<u32>, @builtin(local_invocation_index) local_invocation_index: u32, @builtin(global_invocation_id) global_invocation_id: vec3<u32>, @builtin(workgroup_id) workgroup_id: vec3<u32>) {
    let total = arrayLength(&meshlets.meshlets);
    let meshlet_index = global_invocation_id.x;
    if (meshlet_index >= total) {
        return;
    }
    let mesh_index = commands.commands[meshlet_index].base_instance;

    let accept = cone_culling(meshlets.meshlets[meshlet_index], meshes.meshes[mesh_index], constant_data.view[3].xyz);
    if (!accept) {
        commands.commands[meshlet_index].vertex_count = 0u;
        commands.commands[meshlet_index].instance_count = 0u;
        commands.commands[meshlet_index].base_index = 0u;
        commands.commands[meshlet_index].vertex_offset = 0;
        commands.commands[meshlet_index].base_instance = 0u;
    }
}