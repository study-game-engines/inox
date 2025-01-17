const HIT_EPSILON: f32 = 0.0001;
const INVALID_NODE: i32 = -1;
const MAX_STEPS: u32 = 1024u;

struct Result {
    distance: f32,
    visibility_id: u32,
}


fn intersect_aabb(r: Ray, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> f32 
{     
    let inverse_dir = 1. / r.direction;
    let v_min = (aabb_min - r.origin) * inverse_dir;
    let v_max = (aabb_max - r.origin) * inverse_dir;

    let t_min = min(v_min, v_max);
    let t_max = max(v_min, v_max);

    let t_near = max(max(t_min.x, t_min.y), max(t_min.x, t_min.z));
    let t_far = min(min(t_max.x, t_max.y), min(t_max.x, t_max.z));

    if (t_near > t_far || t_far < 0.) 
    { 
        return MAX_FLOAT; 
    }
    if (t_near > 0.) {
        return t_near;
    } else {
        return t_far;
    } 
}


fn intersect_triangle(r: Ray, v0: vec3<f32>, v1: vec3<f32>, v2: vec3<f32>) -> f32
{
    let e1 = v1 - v0;
    let e2 = v2 - v0;

    let p = cross(r.direction, e2);
    let det = dot(e1, p);
    
    if (abs(det) < HIT_EPSILON) { return MAX_FLOAT; }

    // Computes Barycentric coordinates.
    let inv_det = 1. / det;
    let t1 = r.origin - v0;    
    let u = dot(t1, p) * inv_det;
    if (u < 0. || u > 1.) { return MAX_FLOAT; }
    
    let q = cross(t1, e1);
    let v = dot(r.direction, q) * inv_det;
    if (v < 0. || u + v > 1.) { return MAX_FLOAT; }

    let t2 = dot(e2, q) * inv_det;
    if (t2 < 0.) { return MAX_FLOAT; }
    return t2;
}

fn intersect_meshlet_primitive(r: Ray, mesh_id: u32, meshlet_id: u32, primitive_id: u32) -> f32 {
    let mesh = &meshes.data[mesh_id];    
    let node = &bhv.data[u32((*mesh).bhv_index)];  
    
    let meshlet = &meshlets.data[meshlet_id];
    
    let index_offset = (*mesh).indices_offset + (*meshlet).indices_offset + (primitive_id * 3u);
    let i1 = indices.data[index_offset];
    let i2 = indices.data[index_offset + 1u];
    let i3 = indices.data[index_offset + 2u];

    let v1 = &vertices.data[(*mesh).vertex_offset + i1];
    let v2 = &vertices.data[(*mesh).vertex_offset + i2];
    let v3 = &vertices.data[(*mesh).vertex_offset + i3];
    
    let oobb_size = (*node).max - (*node).min;
    
    let p1 = (*node).min + decode_as_vec3(positions.data[(*v1).position_and_color_offset]) * oobb_size;
    let p2 = (*node).min + decode_as_vec3(positions.data[(*v2).position_and_color_offset]) * oobb_size;
    let p3 = (*node).min + decode_as_vec3(positions.data[(*v3).position_and_color_offset]) * oobb_size;
    
    return intersect_triangle(r, p1, p2, p3);
}

fn is_meshlet_visible(meshlet_id: u32) -> bool
{
    let group_index = meshlet_id / 32u;
    let local_index = meshlet_id % 32u;

    let bits = atomicLoad(&culling_result[group_index]);
    let shift = 1u << local_index;
    return (bits & shift) > 0u;
}

fn is_meshlet_frontfacing(r: Ray, mesh_id: u32, meshlet_id: u32) -> bool 
{    
    let mesh = &meshes.data[mesh_id];   
    let meshlet = &meshlets.data[meshlet_id];
    let cone_culling = &meshlets_culling.data[meshlet_id];
    let cone_axis_cutoff = unpack4x8snorm((*cone_culling).cone_axis_cutoff);

    let direction = (*cone_culling).center - r.origin;
    let is_front_facing = dot(normalize(direction), cone_axis_cutoff.xyz) < (cone_axis_cutoff.w * length(direction));
    return is_front_facing;
}

fn intersect_meshlet(r: Ray, mesh_id: u32, meshlet_id: u32, far_plane: f32) -> Result {
    var nearest = far_plane;  
    var visibility_id = 0u;
    
    let mesh = &meshes.data[mesh_id];   
    let meshlet = &meshlets.data[meshlet_id];
    let node = &bhv.data[u32((*mesh).bhv_index)];  
    let primitive_count = (*meshlet).indices_count / 3u;
    for(var primitive_id = 0u; primitive_id < primitive_count; primitive_id = primitive_id + 1u)
    {       
        let hit = intersect_meshlet_primitive(r, mesh_id, meshlet_id, primitive_id);
        if (hit < nearest) {
            visibility_id = (meshlet_id + 1u) << 8u | primitive_id;
            nearest = hit;
        }
    }
    return Result(nearest, visibility_id);
}

fn traverse_meshlets(r: Ray, mesh_id: u32, far_plane: f32) -> Result {
    let mesh = &meshes.data[mesh_id];    
    var nearest = far_plane;  
    var visibility_id = 0u;
    for(var i = 0u; i < (*mesh).meshlets_count; i = i + 1u)
    {
        let meshlet_id = (*mesh).meshlets_offset + i;             
        if (is_meshlet_visible(meshlet_id)) {
            let hit = intersect_meshlet(r, mesh_id, meshlet_id, nearest);
            if (hit.distance < nearest) {
                visibility_id = hit.visibility_id;
                nearest = hit.distance;
            }
        }
    }
    return Result(nearest, visibility_id);
}

fn traverse_forced_all_bhv_nodes(r: Ray, mesh_id: u32, bhv_start: i32, bhv_end: i32, far_plane: f32) -> Result {
    let mesh = &meshes.data[mesh_id];    
    let mesh_bhv_index = i32((*mesh).bhv_index);
    var bhv_index = bhv_start;
    let nodes_count = i32(arrayLength(&bhv.data));
    var nearest = far_plane;  
    var visibility_id = 0u;

    var steps = 0u;
    while (bhv_index >= bhv_start && bhv_index < bhv_end && bhv_index < nodes_count)
    { 
        let node = &bhv.data[u32(bhv_index)];   
        let intersection = intersect_aabb(r, (*node).min, (*node).max);
        if (intersection < nearest) {
            if ((*node).reference >= 0) {
                //leaf node
                let meshlet_id = (*mesh).meshlets_offset + u32((*node).reference);                
                if (is_meshlet_visible(meshlet_id)) {
                    let hit = intersect_meshlet(r, mesh_id, meshlet_id, nearest);
                    if (hit.distance < nearest) {
                        visibility_id = hit.visibility_id;
                        nearest = hit.distance;
                    }
                }
            }
        }       
        bhv_index = bhv_index + 1;
    }
    return Result(nearest, visibility_id);
}

fn traverse_bhv_of_meshlets(r: Ray, mesh_id: u32, far_plane: f32) -> Result {
    let mesh = &meshes.data[mesh_id];    
    var bhv_index = i32((*mesh).bhv_index);    
    let mesh_bhv_index = bhv_index;
    var nearest = far_plane;  
    var visibility_id = 0u;

    var steps = 0u;
    while (bhv_index >= 0 && steps < MAX_STEPS)
    { 
        let node = &bhv.data[u32(bhv_index)];   
        let intersection = intersect_aabb(r, (*node).min, (*node).max);
        if (intersection < nearest) {
            if ((*node).reference < 0) {
                //inner node
                bhv_index = bhv_index + 1;
                steps = steps + 1u;
                continue;  
            }
            //leaf node
            let meshlet_id = (*mesh).meshlets_offset + u32((*node).reference);             
            if (is_meshlet_visible(meshlet_id)) {
                let hit = intersect_meshlet(r, mesh_id, meshlet_id, nearest);
                if (hit.distance < nearest) {
                    visibility_id = hit.visibility_id;
                    nearest = hit.distance;
                }         
            }
        }        
        bhv_index = (*node).miss;
        if bhv_index >= 0 {
            bhv_index += mesh_bhv_index;
        }
        steps = steps + 1u;
    }
    return Result(nearest, visibility_id);
}


