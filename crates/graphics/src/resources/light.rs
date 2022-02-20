use std::path::{Path, PathBuf};

use inox_math::Vector3;
use inox_messenger::MessageHubRc;
use inox_resources::{
    DataTypeResource, ResourceId, ResourceTrait, SerializableResource, SharedData, SharedDataRc,
};
use inox_serialize::{inox_serializable::SerializableRegistryRc, read_from_file, SerializeFile};

use crate::{LightData, INVALID_INDEX};

pub type LightId = ResourceId;

#[derive(Clone)]
pub struct OnLightCreateData {
    pub position: Vector3,
}

#[derive(Clone)]
pub struct Light {
    filepath: PathBuf,
    data: LightData,
    uniform_index: i32,
    is_active: bool,
}

impl SerializableResource for Light {
    fn path(&self) -> &Path {
        self.filepath.as_path()
    }

    fn set_path(&mut self, path: &Path) {
        self.filepath = path.to_path_buf();
    }

    fn extension() -> &'static str {
        LightData::extension()
    }
}
impl DataTypeResource for Light {
    type DataType = LightData;
    type OnCreateData = OnLightCreateData;

    fn new(_id: ResourceId, _shared_data: &SharedDataRc, _message_hub: &MessageHubRc) -> Self {
        Self {
            filepath: PathBuf::new(),
            data: LightData::default(),
            uniform_index: INVALID_INDEX,
            is_active: true,
        }
    }

    fn is_initialized(&self) -> bool {
        self.uniform_index != INVALID_INDEX
    }

    fn invalidate(&mut self) -> &mut Self {
        self.uniform_index = INVALID_INDEX;
        self
    }

    fn deserialize_data(
        path: &std::path::Path,
        registry: &SerializableRegistryRc,
        f: Box<dyn FnMut(Self::DataType) + 'static>,
    ) {
        read_from_file::<Self::DataType>(path, registry, f);
    }

    fn on_create(
        &mut self,
        _shared_data_rc: &SharedDataRc,
        _message_hub: &MessageHubRc,
        _id: &LightId,
        on_create_data: Option<&<Self as ResourceTrait>::OnCreateData>,
    ) {
        if let Some(on_create_data) = on_create_data {
            self.set_position(on_create_data.position);
        }
    }
    fn on_destroy(
        &mut self,
        _shared_data: &SharedData,
        _message_hub: &MessageHubRc,
        _id: &LightId,
    ) {
    }

    fn create_from_data(
        shared_data: &SharedDataRc,
        message_hub: &MessageHubRc,
        id: ResourceId,
        data: Self::DataType,
    ) -> Self
    where
        Self: Sized,
    {
        let mut light = Self::new(id, shared_data, message_hub);
        light.data = data;
        light
    }
}

impl Light {
    #[inline]
    pub fn set_position(&mut self, position: Vector3) -> &mut Self {
        self.data.position = position.into();
        self
    }

    #[inline]
    pub fn data(&self) -> &LightData {
        &self.data
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut LightData {
        &mut self.data
    }

    #[inline]
    pub fn set_active(&mut self, is_active: bool) -> &mut Self {
        self.is_active = is_active;
        self
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn update_uniform(&mut self, uniform_index: u32, data: &mut LightData) {
        self.uniform_index = uniform_index as _;
        *data = self.data;
    }
    pub fn uniform_index(&self) -> i32 {
        self.uniform_index
    }
}
