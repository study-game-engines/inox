use std::{
    fs::{self, create_dir_all, File},
    io::{Seek, SeekFrom},
    mem::size_of,
    path::{Path, PathBuf},
};

use crate::{need_to_binarize, to_local_path, ExtensionHandler};
use gltf::{
    accessor::{DataType, Dimensions},
    buffer::{Source, View},
    camera::Projection,
    image::Source as ImageSource,
    khr_lights_punctual::{Kind, Light},
    material::AlphaMode,
    mesh::Mode,
    Accessor, Camera, Gltf, Node, Primitive, Semantic, Texture,
};

use inox_graphics::{
    DrawVertex, LightData, LightType, MaterialAlphaMode, MaterialData, MeshData, MeshletData,
    TextureType, MAX_TEXTURE_COORDS_SETS,
};
use inox_log::debug_log;
use inox_math::{
    pack_4_f32_to_unorm, quantize_half, quantize_unorm, Mat4Ops, Matrix4, NewAngle, Parser,
    Radians, VecBase, Vector2, Vector3, Vector4, Vector4h,
};

use inox_nodes::LogicData;
use inox_resources::{to_slice, SharedDataRc};
use inox_scene::{CameraData, ObjectData, SceneData};
use inox_serialize::{
    deserialize, inox_serializable::SerializableRegistryRc, Deserialize, Serialize, SerializeFile,
};

const GLTF_EXTENSION: &str = "gltf";

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "inox_serialize")]
struct ExtraData {
    name: String,
    #[serde(rename = "type")]
    typename: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "inox_serialize")]
struct ExtraProperties {
    logic: ExtraData,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "inox_serialize")]
struct Extras {
    inox_properties: ExtraProperties,
}

#[derive(Clone)]
struct GltfVertex {
    position: Vector3,
    normal: Vector3,
    color: Vector4,
    texture_coords: [Vector2; MAX_TEXTURE_COORDS_SETS],
}

impl Default for GltfVertex {
    fn default() -> Self {
        Self {
            position: Vector3::default_zero(),
            normal: Vector3::unit_y(),
            color: Vector4::default_one(),
            texture_coords: [Vector2::default_zero(); MAX_TEXTURE_COORDS_SETS],
        }
    }
}

#[derive(PartialEq, Eq)]
enum NodeType {
    Object,
    Camera,
    Light,
}

#[derive(Default)]
pub struct GltfCompiler {
    shared_data: SharedDataRc,
    data_raw_folder: PathBuf,
    data_folder: PathBuf,
    optimize_meshes: bool,
    node_index: usize,
    material_index: usize,
}

impl GltfCompiler {
    pub fn new(
        shared_data: SharedDataRc,
        data_raw_folder: &Path,
        data_folder: &Path,
        optimize_meshes: bool,
    ) -> Self {
        Self {
            shared_data,
            data_raw_folder: data_raw_folder.to_path_buf(),
            data_folder: data_folder.to_path_buf(),
            optimize_meshes,
            node_index: 0,
            material_index: 0,
        }
    }

    fn num_from_type(&mut self, accessor: &Accessor) -> usize {
        match accessor.dimensions() {
            Dimensions::Vec2 => 2,
            Dimensions::Vec3 => 3,
            Dimensions::Vec4 => 4,
            Dimensions::Mat2 => 4,
            Dimensions::Mat3 => 9,
            Dimensions::Mat4 => 16,
            _ => 1,
        }
    }
    fn bytes_from_dimension(&mut self, accessor: &Accessor) -> usize {
        match accessor.data_type() {
            DataType::F32 | DataType::U32 => 4,
            DataType::U16 | DataType::I16 => 2,
            _ => 1,
        }
    }

    fn read_accessor_from_path<T>(&mut self, path: &Path, accessor: &Accessor) -> Option<Vec<T>>
    where
        T: Parser,
    {
        let view = if let Some(sparse) = accessor.sparse() {
            Some(sparse.values().view())
        } else {
            accessor.view()
        };
        if let Some(view) = view {
            if let Some(parent_folder) = path.parent() {
                match view.buffer().source() {
                    Source::Uri(local_path) => {
                        let filepath = parent_folder.to_path_buf().join(local_path);
                        if let Ok(mut file) = fs::File::open(filepath) {
                            return Some(self.read_from_file::<T>(&mut file, &view, accessor));
                        } else {
                            eprintln!("Unable to open file: {local_path}");
                        }
                    }
                    Source::Bin => {}
                }
            }
        }
        None
    }

    fn read_from_file<T>(&mut self, file: &mut File, view: &View, accessor: &Accessor) -> Vec<T>
    where
        T: Parser,
    {
        let count = accessor.count();
        let view_offset = view.offset();
        let accessor_offset = accessor.offset();
        let starting_offset = view_offset + accessor_offset;
        let view_stride = view.stride().unwrap_or(0);
        let type_stride = T::size();
        let stride = if view_stride > type_stride {
            view_stride - type_stride
        } else {
            0
        };
        let mut result = Vec::new();
        file.seek(SeekFrom::Start(starting_offset as _)).ok();
        for _i in 0..count {
            let v = T::parse(file);
            result.push(v);
            file.seek(SeekFrom::Current(stride as _)).ok();
        }
        result
    }

    fn extract_indices(&mut self, path: &Path, primitive: &Primitive) -> Vec<u32> {
        debug_assert!(primitive.mode() == Mode::Triangles);
        let mut indices = Vec::new();
        if let Some(accessor) = primitive.indices() {
            let num = self.num_from_type(&accessor);
            let num_bytes = self.bytes_from_dimension(&accessor);
            debug_assert!(num == 1);
            if num_bytes == 1 {
                if let Some(ind) = self.read_accessor_from_path::<u8>(path, &accessor) {
                    indices = ind.iter().map(|e| *e as u32).collect();
                }
            } else if num_bytes == 2 {
                if let Some(ind) = self.read_accessor_from_path::<u16>(path, &accessor) {
                    indices = ind.iter().map(|e| *e as u32).collect();
                }
            } else if let Some(ind) = self.read_accessor_from_path::<u32>(path, &accessor) {
                indices = ind;
            }
        }
        indices
    }

    fn extract_vertices(&mut self, path: &Path, primitive: &Primitive) -> Vec<GltfVertex> {
        let mut vertices = Vec::new();
        for (_attribute_index, (semantic, accessor)) in primitive.attributes().enumerate() {
            //debug_log!("Attribute[{}]: {:?}", _attribute_index, semantic);
            match semantic {
                Semantic::Positions => {
                    let num = self.num_from_type(&accessor);
                    let num_bytes = self.bytes_from_dimension(&accessor);
                    debug_assert!(num == 3 && num_bytes == 4);
                    if let Some(pos) = self.read_accessor_from_path::<Vector3>(path, &accessor) {
                        if vertices.is_empty() {
                            vertices.resize_with(pos.len(), GltfVertex::default);
                        }
                        pos.iter().enumerate().for_each(|(i, p)| {
                            vertices[i].position = *p;
                        });
                    }
                }
                Semantic::Normals => {
                    let num = self.num_from_type(&accessor);
                    let num_bytes = self.bytes_from_dimension(&accessor);
                    debug_assert!(num == 3 && num_bytes == 4);
                    if let Some(norm) = self.read_accessor_from_path::<Vector3>(path, &accessor) {
                        if vertices.is_empty() {
                            vertices.resize_with(norm.len(), GltfVertex::default);
                        }
                        norm.iter().enumerate().for_each(|(i, n)| {
                            vertices[i].normal = *n;
                        });
                    }
                }
                /*
                Semantic::Tangents => {
                    let num = self.num_from_type(&accessor);
                    let num_bytes = self.bytes_from_dimension(&accessor);
                    debug_assert!(num == 4 && num_bytes == 4);
                    if let Some(tang) = self.read_accessor_from_path::<Vector4>(path, &accessor) {
                        mesh_data.tangents.extend_from_slice(tang.as_slice());
                        mesh_data.vertices.resize(tang.len(), DrawVertex::default());
                        mesh_data
                            .vertices
                            .iter_mut()
                            .enumerate()
                            .for_each(|(i, v)| {
                                v.tangent_offset = i as _;
                            });
                    }
                }
                */
                Semantic::Colors(_color_index) => {
                    let num = self.num_from_type(&accessor);
                    let num_bytes = self.bytes_from_dimension(&accessor);
                    debug_assert!(num == 4);
                    if num_bytes == 2 {
                        debug_assert!(num_bytes == 2);
                        if let Some(col) = self.read_accessor_from_path::<Vector4h>(path, &accessor)
                        {
                            if vertices.is_empty() {
                                vertices.resize_with(col.len(), GltfVertex::default);
                            }
                            col.iter().enumerate().for_each(|(i, c)| {
                                vertices[i].color =
                                    [c.x as f32, c.y as f32, c.z as f32, c.z as f32].into();
                            });
                        }
                    } else {
                        debug_assert!(num_bytes == 4);
                        if let Some(col) = self.read_accessor_from_path::<Vector4>(path, &accessor)
                        {
                            if vertices.is_empty() {
                                vertices.resize_with(col.len(), GltfVertex::default);
                            }
                            col.iter().enumerate().for_each(|(i, c)| {
                                vertices[i].color = *c;
                            });
                        }
                    }
                }
                Semantic::TexCoords(texture_index) => {
                    if texture_index >= MAX_TEXTURE_COORDS_SETS as _ {
                        eprintln!(
                            "ERROR: Texture coordinate set {texture_index} is out of range (max {MAX_TEXTURE_COORDS_SETS})",
                            
                        );
                        continue;
                    }
                    let num = self.num_from_type(&accessor);
                    let num_bytes = self.bytes_from_dimension(&accessor);
                    debug_assert!(num == 2 && num_bytes == 4);
                    if let Some(tex) = self.read_accessor_from_path::<Vector2>(path, &accessor) {
                        if vertices.is_empty() {
                            vertices.resize_with(tex.len(), GltfVertex::default);
                        }
                        tex.iter().enumerate().for_each(|(i, t)| {
                            vertices[i].texture_coords[texture_index as usize] = *t;
                        });
                    }
                }
                _ => {}
            }
        }
        vertices
    }

    fn optimize_mesh(&self, vertices: &mut Vec<GltfVertex>, indices: &mut Vec<u32>) {
        if self.optimize_meshes {
            let (num_vertices, vertices_remap_table) =
                meshopt::generate_vertex_remap(vertices.as_slice(), Some(indices.as_slice()));

            let new_vertices = meshopt::remap_vertex_buffer(
                vertices.as_slice(),
                num_vertices,
                vertices_remap_table.as_slice(),
            );
            let new_indices = meshopt::remap_index_buffer(
                Some(indices.as_slice()),
                num_vertices,
                vertices_remap_table.as_slice(),
            );

            let mut new_indices =
                meshopt::optimize_vertex_cache(new_indices.as_slice(), num_vertices);

            let vertices_bytes = to_slice(vertices.as_slice());
            let vertex_stride = size_of::<GltfVertex>();
            let vertex_data_adapter =
                meshopt::VertexDataAdapter::new(vertices_bytes, vertex_stride, 0);
            meshopt::optimize_overdraw_in_place(
                new_indices.as_mut_slice(),
                vertex_data_adapter.as_ref().unwrap(),
                1.01,
            );
            let new_vertices =
                meshopt::optimize_vertex_fetch(new_indices.as_mut_slice(), new_vertices.as_slice());

            *vertices = new_vertices;
            *indices = new_indices;
        }
    }

    fn compute_meshlets(
        &self,
        vertices: &[GltfVertex],
        indices: &mut Vec<u32>,
    ) -> Vec<MeshletData> {
        let mut new_meshlets = Vec::new();
        let vertices_bytes = to_slice(vertices);
        let vertex_stride = size_of::<GltfVertex>();
        let vertex_data_adapter = meshopt::VertexDataAdapter::new(vertices_bytes, vertex_stride, 0);
        let max_vertices = 64;
        let max_triangles = 124;
        let cone_weight = 0.7;
        let meshlets = meshopt::build_meshlets(
            indices,
            vertex_data_adapter.as_ref().unwrap(),
            max_vertices,
            max_triangles,
            cone_weight,
        );
        if !meshlets.meshlets.is_empty() {
            let mut new_indices = Vec::new();
            for m in meshlets.iter() {
                let bounds =
                    meshopt::compute_meshlet_bounds(m, vertex_data_adapter.as_ref().unwrap());
                let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
                let mut max = Vector3::new(-f32::MAX, -f32::MAX, -f32::MAX);
                m.vertices.iter().for_each(|i| {
                    let v = &vertices[*i as usize];
                    min = min.min(v.position);
                    max = max.max(v.position);
                });
                let index_offset = new_indices.len();
                debug_assert!(
                    m.triangles.len() % 3 == 0,
                    "meshlet indices count {} is not divisible by 3",
                    m.triangles.len()
                );
                m.triangles.iter().for_each(|v_i| {
                    new_indices.push(m.vertices[*v_i as usize]);
                });
                new_meshlets.push(MeshletData {
                    indices_offset: index_offset as _,
                    indices_count: m.triangles.len() as _,
                    aabb_max: max,
                    aabb_min: min,
                    cone_axis: bounds.cone_axis.into(),
                    cone_angle: bounds.cone_cutoff,
                    cone_center: bounds.center.into(),
                });
            }
            debug_assert!(
                new_indices.len() % 3 == 0,
                "new indices count {} is not divisible by 3",
                new_indices.len()
            );
            *indices = new_indices;
        } else {
            let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
            let mut max = Vector3::new(-f32::MAX, -f32::MAX, -f32::MAX);
            vertices.iter().for_each(|v| {
                min = min.min(v.position);
                max = max.max(v.position);
            });
            let meshlet = MeshletData {
                indices_offset: 0,
                indices_count: indices.len() as _,
                aabb_max: max,
                aabb_min: min,
                ..Default::default()
            };
            new_meshlets.push(meshlet);
        }
        new_meshlets
    }

    fn process_mesh_data(
        &mut self,
        path: &Path,
        mesh_name: &str,
        primitive: &Primitive,
        material_path: &Path,
    ) -> PathBuf {
        let mut vertices = self.extract_vertices(path, primitive);
        let mut indices = self.extract_indices(path, primitive);
        self.optimize_mesh(&mut vertices, &mut indices);

        let mut mesh_data = MeshData::default();
        mesh_data
            .vertices
            .resize(vertices.len(), DrawVertex::default());

        mesh_data.aabb_max = Vector3::new(-f32::INFINITY, -f32::INFINITY, -f32::INFINITY);
        mesh_data.aabb_min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        vertices.iter().for_each(|v| {
            mesh_data.aabb_max = mesh_data.aabb_max.max(v.position);
            mesh_data.aabb_min = mesh_data.aabb_min.min(v.position);
        });
        let size = mesh_data.aabb_max - mesh_data.aabb_min;

        vertices.iter().enumerate().for_each(|(i, vertex)| {
            let mut v = vertex.position - mesh_data.aabb_min;
            v.x /= size.x;
            v.y /= size.y;
            v.z /= size.z;
            let vx = quantize_unorm(v.x, 10);
            let vy = quantize_unorm(v.y, 10);
            let vz = quantize_unorm(v.z, 10);
            let position = vx << 20 | vy << 10 | vz;
            mesh_data.positions.push(position);
            mesh_data.vertices[i].position_and_color_offset = (mesh_data.positions.len() - 1) as _;

            let color = pack_4_f32_to_unorm(vertex.color);
            mesh_data.colors.push(color);

            let n = vertex.normal;
            let nx = quantize_unorm(n.x, 10);
            let ny = quantize_unorm(n.y, 10);
            let nz = quantize_unorm(n.z, 10);
            let normal = nx << 20 | ny << 10 | nz;
            mesh_data.normals.push(normal);
            mesh_data.vertices[i].normal_offset = (mesh_data.normals.len() - 1) as _;

            let mut uvs = Vec::new();
            vertex.texture_coords.iter().enumerate().for_each(|(j, t)| {
                let u = quantize_half(t.x) as u32;
                let v = (quantize_half(t.y) as u32) << 16;
                uvs.push(u | v);
                mesh_data.vertices[i].uv_offset[j] = (mesh_data.uvs.len() + uvs.len() - 1) as _;
            });
            mesh_data.uvs.extend(uvs.iter());
        });

        mesh_data.indices = indices;

        mesh_data.meshlets = self.compute_meshlets(vertices.as_slice(), &mut mesh_data.indices);

        mesh_data.material = material_path.to_path_buf();

        self.create_file(
            path,
            &mesh_data,
            mesh_name,
            "mesh",
            self.shared_data.serializable_registry(),
        )
    }
    fn process_texture(&mut self, path: &Path, texture: Texture) -> PathBuf {
        if let ImageSource::Uri {
            uri,
            mime_type: _, /* fields */
        } = texture.source().source()
        {
            if let Some(parent_folder) = path.parent() {
                let parent_path = parent_folder.to_str().unwrap().to_string();
                let filepath = PathBuf::from(parent_path).join(uri);
                let path = to_local_path(
                    filepath.as_path(),
                    self.data_raw_folder.as_path(),
                    self.data_folder.as_path(),
                );
                return path;
            }
        }
        PathBuf::new()
    }
    fn process_material_data(&mut self, path: &Path, primitive: &Primitive) -> PathBuf {
        let mut material_data = MaterialData::default();

        let material = primitive.material().pbr_metallic_roughness();
        material_data.base_color = material.base_color_factor().into();
        material_data.roughness_factor = material.roughness_factor();
        material_data.metallic_factor = material.metallic_factor();
        if let Some(info) = material.base_color_texture() {
            material_data.textures[TextureType::BaseColor as usize] =
                self.process_texture(path, info.texture());
            material_data.texcoords_set[TextureType::BaseColor as usize] = info.tex_coord() as _;
        }
        if let Some(info) = material.metallic_roughness_texture() {
            material_data.textures[TextureType::MetallicRoughness as usize] =
                self.process_texture(path, info.texture());
            material_data.texcoords_set[TextureType::MetallicRoughness as usize] =
                info.tex_coord() as _;
        }

        let material = primitive.material();
        if let Some(texture) = material.normal_texture() {
            material_data.textures[TextureType::Normal as usize] =
                self.process_texture(path, texture.texture());
            material_data.texcoords_set[TextureType::Normal as usize] = texture.tex_coord() as _;
        }
        if let Some(texture) = material.emissive_texture() {
            material_data.textures[TextureType::Emissive as usize] =
                self.process_texture(path, texture.texture());
            material_data.texcoords_set[TextureType::Emissive as usize] = texture.tex_coord() as _;
        }
        if let Some(texture) = material.occlusion_texture() {
            material_data.textures[TextureType::Occlusion as usize] =
                self.process_texture(path, texture.texture());
            material_data.texcoords_set[TextureType::Occlusion as usize] = texture.tex_coord() as _;
            material_data.occlusion_strength = texture.strength();
        }
        material_data.alpha_mode = match material.alpha_mode() {
            AlphaMode::Opaque => MaterialAlphaMode::Opaque,
            AlphaMode::Mask => {
                material_data.alpha_cutoff = 0.5;
                MaterialAlphaMode::Mask
            }
            AlphaMode::Blend => MaterialAlphaMode::Blend,
        };
        material_data.alpha_cutoff = primitive.material().alpha_cutoff().unwrap_or(1.);
        material_data.emissive_color = [
            primitive.material().emissive_factor()[0],
            primitive.material().emissive_factor()[1],
            primitive.material().emissive_factor()[2],
        ]
        .into();
        if let Some(material) = material.pbr_specular_glossiness() {
            if let Some(texture) = material.specular_glossiness_texture() {
                material_data.textures[TextureType::SpecularGlossiness as usize] =
                    self.process_texture(path, texture.texture());
                material_data.texcoords_set[TextureType::SpecularGlossiness as usize] =
                    texture.tex_coord() as _;
            }
            if let Some(texture) = material.diffuse_texture() {
                material_data.textures[TextureType::Diffuse as usize] =
                    self.process_texture(path, texture.texture());
                material_data.texcoords_set[TextureType::Diffuse as usize] =
                    texture.tex_coord() as _;
            }
            material_data.diffuse_color = material.diffuse_factor().into();
            material_data.specular_color = [
                material.specular_factor()[0],
                material.specular_factor()[1],
                material.specular_factor()[2],
                1.,
            ]
            .into();
        }

        let name = format!("Material_{}", self.material_index);
        self.create_file(
            path,
            &material_data,
            primitive.material().name().unwrap_or(&name),
            "material",
            self.shared_data.serializable_registry(),
        )
    }

    fn process_node(
        &mut self,
        path: &Path,
        node: &Node,
        node_name: &str,
    ) -> Option<(NodeType, PathBuf)> {
        let (node_type, node_path) = self.process_object(path, node, node_name);
        self.node_index += 1;
        Some((node_type, node_path))
    }

    fn process_object(&mut self, path: &Path, node: &Node, node_name: &str) -> (NodeType, PathBuf) {
        let mut object_data = ObjectData::default();
        let object_transform: Matrix4 = Matrix4::from(node.transform().matrix());
        object_data.transform = object_transform;

        if let Some(mesh) = node.mesh() {
            for (primitive_index, primitive) in mesh.primitives().enumerate() {
                let name = format!("{node_name}_Primitive_{primitive_index}");
                let material_path = self.process_material_data(path, &primitive);
                let material_path = to_local_path(
                    material_path.as_path(),
                    self.data_raw_folder.as_path(),
                    self.data_folder.as_path(),
                );
                let mesh_path =
                    self.process_mesh_data(path, &name, &primitive, material_path.as_path());
                let mesh_path = to_local_path(
                    mesh_path.as_path(),
                    self.data_raw_folder.as_path(),
                    self.data_folder.as_path(),
                );
                object_data.components.push(mesh_path);
            }
        }
        if let Some(camera) = node.camera() {
            let position = object_data.transform.translation();
            let mut matrix =
                Matrix4::from_nonuniform_scale(1., 1., -1.) * object_data.transform.inverse();
            matrix.set_translation(position);
            object_data.transform = matrix;
            let (_, camera_path) = self.process_camera(path, &camera);
            object_data.components.push(to_local_path(
                camera_path.as_path(),
                self.data_raw_folder.as_path(),
                self.data_folder.as_path(),
            ));
        }
        if let Some(light) = node.light() {
            let (_, light_path) = self.process_light(path, &light);
            object_data.components.push(to_local_path(
                light_path.as_path(),
                self.data_raw_folder.as_path(),
                self.data_folder.as_path(),
            ));
        }
        if let Some(extras) = node.extras() {
            if let Ok(extras) = deserialize::<Extras>(
                extras.to_string().as_str(),
                self.shared_data.serializable_registry(),
            ) {
                if !extras.inox_properties.logic.name.is_empty() {
                    let mut path = path
                        .parent()
                        .unwrap()
                        .join(LogicData::extension())
                        .to_str()
                        .unwrap()
                        .to_string();
                    path.push_str(
                        format!(
                            "\\{}.{}",
                            extras.inox_properties.logic.name,
                            LogicData::extension()
                        )
                        .as_str(),
                    );
                    object_data.components.push(to_local_path(
                        PathBuf::from(path).as_path(),
                        self.data_raw_folder.as_path(),
                        self.data_folder.as_path(),
                    ));
                }
            }
        }

        for (child_index, child) in node.children().enumerate() {
            let name = format!("Node_{}_Child_{}", self.node_index, child_index);
            if let Some(camera) = child.camera() {
                object_data.transform =
                    object_data.transform * Matrix4::from(child.transform().matrix());
                let position = object_data.transform.translation();
                let mut matrix =
                    Matrix4::from_nonuniform_scale(1., 1., -1.) * object_data.transform.inverse();
                matrix.set_translation(position);
                object_data.transform = matrix;
                let (_, camera_path) = self.process_camera(path, &camera);
                object_data.components.push(to_local_path(
                    camera_path.as_path(),
                    self.data_raw_folder.as_path(),
                    self.data_folder.as_path(),
                ));
            } else if let Some((node_type, node_path)) =
                self.process_node(path, &child, child.name().unwrap_or(&name))
            {
                if node_type == NodeType::Object {
                    let node_path = to_local_path(
                        node_path.as_path(),
                        self.data_raw_folder.as_path(),
                        self.data_folder.as_path(),
                    );
                    object_data.children.push(node_path);
                }
            }
        }

        (
            NodeType::Object,
            self.create_file(
                path,
                &object_data,
                node_name,
                "object",
                self.shared_data.serializable_registry(),
            ),
        )
    }

    fn process_light(&mut self, path: &Path, light: &Light) -> (NodeType, PathBuf) {
        let mut light_data = LightData {
            color: [light.color()[0], light.color()[1], light.color()[2], 1.],
            intensity: light.intensity().max(1.),
            range: light.range().unwrap_or(10.),
            ..Default::default()
        };
        match light.kind() {
            Kind::Directional => {
                light_data.light_type = LightType::Directional as _;
            }
            Kind::Point => {
                light_data.light_type = LightType::Point as _;
            }
            Kind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            } => {
                light_data.light_type = LightType::Spot as _;
                light_data.inner_cone_angle = inner_cone_angle;
                light_data.outer_cone_angle = outer_cone_angle;
            }
        }

        let name = format!("Node_{}_Light_{}", self.node_index, light.index());
        (
            NodeType::Light,
            self.create_file(
                path,
                &light_data,
                &name,
                "light",
                self.shared_data.serializable_registry(),
            ),
        )
    }

    fn process_camera(&mut self, path: &Path, camera: &Camera) -> (NodeType, PathBuf) {
        let mut camera_data = CameraData::default();
        match camera.projection() {
            Projection::Perspective(p) => {
                camera_data.aspect_ratio = p.aspect_ratio().unwrap_or(1920. / 1080.);
                camera_data.near = p.znear();
                camera_data.far = p.zfar().unwrap_or(camera_data.near + 1000.);
                camera_data.fov = Radians::new(p.yfov()).into();
            }
            Projection::Orthographic(o) => {
                camera_data.near = o.znear();
                camera_data.far = o.zfar();
            }
        }
        let name = format!("Node_{}_Camera_{}", self.node_index, camera.index());

        (
            NodeType::Camera,
            self.create_file(
                path,
                &camera_data,
                &name,
                "camera",
                self.shared_data.serializable_registry(),
            ),
        )
    }

    pub fn process_path(&mut self, path: &Path) {
        if let Ok(gltf) = Gltf::open(path) {
            for scene in gltf.scenes() {
                let mut scene_data = SceneData::default();
                let scene_name = path
                    .parent()
                    .unwrap()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap();

                let new_path = self.compute_path_name::<SceneData>(path, scene_name, "");
                if need_to_binarize(path, new_path.as_path()) {
                    self.material_index = 0;
                    self.node_index = 0;
                    for node in scene.nodes() {
                        let name = format!("Node_{}", self.node_index);
                        if let Some((node_type, node_path)) =
                            self.process_node(path, &node, node.name().unwrap_or(&name))
                        {
                            let node_path = to_local_path(
                                node_path.as_path(),
                                self.data_raw_folder.as_path(),
                                self.data_folder.as_path(),
                            );
                            match node_type {
                                NodeType::Camera => {
                                    scene_data.cameras.push(node_path);
                                }
                                NodeType::Object => {
                                    scene_data.objects.push(node_path);
                                }
                                NodeType::Light => {
                                    scene_data.lights.push(node_path);
                                }
                            }
                        }
                    }

                    self.create_file(
                        path,
                        &scene_data,
                        scene_name,
                        "",
                        self.shared_data.serializable_registry(),
                    );
                }
            }
        }
    }

    fn compute_path_name<T>(&self, path: &Path, new_name: &str, folder: &str) -> PathBuf
    where
        T: Serialize + SerializeFile + Clone + 'static,
    {
        let filename = path.file_name().unwrap().to_str().unwrap();
        let destination_ext = format!("{}.{}", new_name, T::extension());
        let mut filepath = path.parent().unwrap().to_path_buf();
        if !folder.is_empty() {
            filepath = filepath.join(folder);
        }
        filepath = filepath.join(filename);
        let mut from_source_to_compiled = filepath.to_str().unwrap().to_string();
        from_source_to_compiled = from_source_to_compiled.replace(
            self.data_raw_folder
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap(),
            self.data_folder.canonicalize().unwrap().to_str().unwrap(),
        );
        from_source_to_compiled =
            from_source_to_compiled.replace(filename, destination_ext.as_str());

        PathBuf::from(from_source_to_compiled)
    }

    fn create_file<T>(
        &self,
        path: &Path,
        data: &T,
        new_name: &str,
        folder: &str,
        serializable_registry: &SerializableRegistryRc,
    ) -> PathBuf
    where
        T: Serialize + SerializeFile + Clone + 'static,
    {
        let new_path = self.compute_path_name::<T>(path, new_name, folder);
        if !new_path.exists() {
            let result = create_dir_all(new_path.parent().unwrap());
            debug_assert!(result.is_ok());
        }
        if need_to_binarize(path, new_path.as_path()) {
            debug_log!("Serializing {:?}", new_path);
            data.save_to_file(new_path.as_path(), serializable_registry);
        }
        new_path
    }
}

impl ExtensionHandler for GltfCompiler {
    fn on_changed(&mut self, path: &Path) {
        if let Some(ext) = path.extension() {
            let extension = ext.to_str().unwrap().to_string();
            if extension.as_str() == GLTF_EXTENSION {
                self.process_path(path);
            }
        }
    }
}
