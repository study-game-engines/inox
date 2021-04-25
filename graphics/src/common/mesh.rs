use super::data_formats::*;
use super::device::*;

const MAX_BUFFER_SIZE: usize = 4096;

#[derive(Clone)]
pub struct Mesh {
    inner: crate::api::backend::mesh::Mesh,
    device: Device,
    is_finalized: bool,
    pub data: MeshData,
}

impl Mesh {
    pub fn create(device: &Device) -> Mesh {
        Self {
            inner: crate::api::backend::mesh::Mesh::default(),
            device: device.clone(),
            is_finalized: false,
            data: MeshData::default(),
        }
    }
    pub fn is_finalized(&self) -> bool {
        self.is_finalized
    }
    pub fn destroy(&mut self) {
        self.inner.delete(&self.device.inner);
    }

    pub fn fill_mesh_with_max_buffers(&mut self) {
        self.data
            .vertices
            .resize_with(MAX_BUFFER_SIZE, VertexData::default);
        self.data.indices.resize_with(MAX_BUFFER_SIZE, u32::default);
    }

    pub fn finalize(&mut self) -> &mut Self {
        if !self.is_finalized {
            self.is_finalized = true;
            if !self.data.vertices.is_empty() {
                self.inner
                    .create_vertex_buffer(&self.device.inner, self.data.vertices.as_slice());
            }
            if !self.data.indices.is_empty() {
                self.inner
                    .create_index_buffer(&self.device.inner, self.data.indices.as_slice());
            }
        }
        self
    }

    pub fn bind_vertices(&mut self, num_vertices: u32) {
        self.inner
            .bind_vertices(&self.device.inner, &self.data.vertices, num_vertices);
    }

    pub fn bind_indices(&mut self, num_indices: u32) {
        self.inner
            .bind_indices(&self.device.inner, &self.data.indices, num_indices);
    }

    pub fn draw(&mut self, num_vertices: u32, num_indices: u32) {
        if !self.data.vertices.is_empty() {
            self.inner.draw(
                &self.device.inner,
                &self.data.vertices,
                num_vertices,
                &self.data.indices,
                num_indices,
            );
        }
    }
}
