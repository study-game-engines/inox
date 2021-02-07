use std::{any::TypeId, collections::HashMap, sync::{Arc, RwLock}};

use super::resource::*;

struct ResourceStorage {
    stored: Vec<Arc<dyn ResourceTrait>>,
}
unsafe impl Send for ResourceStorage {}
unsafe impl Sync for ResourceStorage {}

impl Default for ResourceStorage {
    fn default() -> Self {
        Self{
            stored: Vec::new(),
        }
    }
}

impl ResourceStorage {
    pub fn add_resource<T: 'static>(&mut self, resource: Resource<T>) -> ResourceId {
        let id = resource.id();
        self.stored.push(Arc::new(resource));
        id
    }
    
    pub fn get_resource<T: 'static>(&self, resource_id: ResourceId) -> ResourceRef<T> {
        let item = self.stored.iter().find(|&x|{
            let item: Arc<Resource<T>> = unsafe { std::mem::transmute_copy(x ) };
            let res = unsafe { &*Arc::into_raw(item) };
            res.id() == resource_id
        }).unwrap();
        let item: Arc<Resource<T>> = unsafe { std::mem::transmute_copy(item ) };
        let res = Arc::into_raw(item);
        ResourceRef::new( unsafe{ &*res })
    }
    
    pub fn get_unique_resource<T: 'static>(&self) -> ResourceRef<T> {
        debug_assert!(self.stored.len() == 1, "Trying to get unique resource but multiple resource of same type exists");
        let item = self.stored.first().unwrap();
        let item: Arc<Resource<T>> = unsafe { std::mem::transmute_copy(item ) };
        let res = Arc::into_raw(item);
        ResourceRef::new( unsafe{ &*res })
    }
}

pub struct SharedData {
    resources: HashMap<TypeId, ResourceStorage>
}
unsafe impl Send for SharedData {}
unsafe impl Sync for SharedData {}

impl Default for SharedData {
    fn default() -> Self {
        Self{
            resources: HashMap::new(),
        }
    }
}

impl SharedData {
    pub fn add_resource<T: 'static>(&mut self, data: T) -> ResourceId {
        let vec = self.resources.entry(TypeId::of::<T>()).or_insert(ResourceStorage::default());
        vec.add_resource(Resource::new(data))
    }
    
    pub fn get_resource<T: 'static>(&self, resource_id: ResourceId) -> ResourceRef<T> {
        let vec = self.resources.get(&TypeId::of::<T>()).unwrap();
        vec.get_resource(resource_id)
    }
    
    pub fn get_unique_resource<T: 'static>(&self) -> ResourceRef<T> {
        let vec = self.resources.get(&TypeId::of::<T>()).unwrap();
        vec.get_unique_resource()
    }
}

pub type SharedDataRw = Arc<RwLock<SharedData>>;