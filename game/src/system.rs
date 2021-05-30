use std::path::Path;

use super::config::*;

use nrg_core::*;
use nrg_graphics::*;
use nrg_math::{Matrix4, Vector3};
use nrg_resources::SharedDataRw;
use nrg_serialize::INVALID_UID;

pub struct MySystem {
    id: SystemId,
    shared_data: SharedDataRw,
    config: Config,
    font_id: FontId,
    left_material_id: MaterialId,
    right_material_id: MaterialId,
    left_texture_id: TextureId,
    right_texture_id: TextureId,
    left_mesh_id: MeshId,
    right_mesh_id: MeshId,
    angle: f32,
}

impl MySystem {
    pub fn new(shared_data: &SharedDataRw, config: &Config) -> Self {
        Self {
            id: SystemId::new(),
            shared_data: shared_data.clone(),
            config: config.clone(),
            font_id: INVALID_UID,
            left_material_id: INVALID_UID,
            right_material_id: INVALID_UID,
            left_texture_id: INVALID_UID,
            right_texture_id: INVALID_UID,
            left_mesh_id: INVALID_UID,
            right_mesh_id: INVALID_UID,
            angle: 0.,
        }
    }
}

impl System for MySystem {
    fn id(&self) -> SystemId {
        self.id
    }
    fn init(&mut self) {
        let pipeline_id = PipelineInstance::find_id_from_name(&self.shared_data, "Default");
        self.font_id = FontInstance::create_from_path(
            &self.shared_data,
            pipeline_id,
            self.config.fonts.first().unwrap(),
        );

        self.left_material_id =
            MaterialInstance::create_from_pipeline(&self.shared_data, pipeline_id);
        self.right_material_id =
            MaterialInstance::create_from_pipeline(&self.shared_data, pipeline_id);
        self.left_texture_id = TextureInstance::create_from_path(
            &self.shared_data,
            &Path::new("./data/textures/Test.png"),
        );
        self.right_texture_id = TextureInstance::create_from_path(
            &self.shared_data,
            &Path::new("./data/textures/Test.jpg"),
        );

        let mut mesh = MeshData::default();
        mesh.add_quad_default([0., 0., 1., 1.].into(), 0.);
        self.left_mesh_id = MeshInstance::create(&self.shared_data, mesh);

        let mut mesh = MeshData::default();
        mesh.add_quad(
            [-0.5, -0.5, 0.5, 0.5].into(),
            1.,
            [0.0, 0.0, 1.0, 1.0].into(),
            None,
        );
        self.right_mesh_id = MeshInstance::create(&self.shared_data, mesh);

        MaterialInstance::add_texture(
            &self.shared_data,
            self.left_material_id,
            self.left_texture_id,
        );
        MaterialInstance::add_mesh(&self.shared_data, self.left_material_id, self.left_mesh_id);

        MaterialInstance::add_texture(
            &self.shared_data,
            self.right_material_id,
            self.right_texture_id,
        );
        MaterialInstance::add_mesh(
            &self.shared_data,
            self.right_material_id,
            self.right_mesh_id,
        );
    }

    fn run(&mut self) -> bool {
        let mut left_matrix = Matrix4::from_nonuniform_scale(400., 400., 1.);
        left_matrix[3][0] = 100.;
        left_matrix[3][1] = 100.;

        MeshInstance::set_transform(&self.shared_data, self.left_mesh_id, left_matrix);

        self.angle += 0.33;
        let right_matrix = Matrix4::from_translation(Vector3::new(1000., 800., 0.))
            * Matrix4::from_angle_z(nrg_math::Rad::from(nrg_math::Deg(self.angle)))
            * Matrix4::from_nonuniform_scale(400., 600., 1.);

        MeshInstance::set_transform(&self.shared_data, self.right_mesh_id, right_matrix);
        MaterialInstance::set_outline_color(
            &self.shared_data,
            self.right_material_id,
            [1., 1., 0., 10.].into(),
        );

        true
    }
    fn uninit(&mut self) {}
}
