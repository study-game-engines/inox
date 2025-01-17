#[macro_export]
macro_rules! implement_pin {
    ($Type:ident) => {
        #[inox_serializable::serializable]
        impl $crate::Pin for $Type {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            fn get_type_id(&self) -> std::any::TypeId {
                std::any::TypeId::of::<$Type>()
            }
            fn get_type_name(&self) -> &'static str {
                std::any::type_name::<$Type>()
                    .split(':')
                    .collect::<Vec<&str>>()
                    .last()
                    .unwrap()
            }
            fn duplicate(&self) -> Box<dyn $crate::Pin> {
                Box::new(self.clone())
            }
        }
        impl $crate::PinType for $Type {
            fn type_id(&self) -> std::any::TypeId {
                std::any::TypeId::of::<$Type>()
            }
            fn resolve_pin(
                &self,
                from_node: &Node,
                from_pin: &str,
                to_node: &mut Node,
                to_pin: &str,
            ) {
                if let Some(i) = to_node.get_input_mut::<$Type>(to_pin) {
                    if let Some(o) = from_node.get_output::<$Type>(from_pin) {
                        *i = o.clone();
                    }
                }
            }
            fn copy_from(&mut self, node: &Node, output_pin: &$crate::PinId) {
                if let Some(o) = node.output::<$Type>(output_pin) {
                    *self = o.clone();
                }
            }
        }
    };
}

#[macro_export]
macro_rules! implement_node {
    ($Type:ident, $NodeField:ident, $Category:expr, $Description:expr, $ExecutionType:expr) => {
        #[inox_serializable::serializable]
        impl $crate::NodeTrait for $Type {
            fn get_type() -> &'static str
            where
                Self: Sized,
            {
                stringify!($Type)
            }
            fn category() -> &'static str
            where
                Self: Sized,
            {
                $Category
            }
            fn description() -> &'static str
            where
                Self: Sized,
            {
                $Description
            }
            fn node(&self) -> &$crate::Node {
                &self.$NodeField
            }
            fn node_mut(&mut self) -> &mut $crate::Node {
                &mut self.$NodeField
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            //need an on_update() function in the node
            fn execute(
                &mut self,
                pin: &$crate::PinId,
                context: &$crate::LogicContext,
            ) -> NodeState {
                self.on_update(pin, context)
            }
            fn execytion_type(&self) -> $crate::NodeExecutionType {
                $ExecutionType
            }
            fn duplicate(&self) -> Box<dyn $crate::NodeTrait + Send + Sync>
            where
                Self: Sized,
            {
                Box::new(self.clone())
            }
            fn serialize_node(&self, registry: &inox_serializable::SerializableRegistryRc) -> String {
                inox_serialize::serialize(self, registry)
            }
            fn deserialize_node(&self, s: &str, registry: &inox_serializable::SerializableRegistryRc) -> Option<Self>
            where
                Self: Sized,
            {
                if let Ok(n) = inox_serialize::deserialize(s, registry) {
                    return Some(n);
                }
                None
            }
        }
    };
}

#[macro_export]
macro_rules! implement_logic_context_data {
    ($Type:ident) => {
        impl $crate::LogicContextData for $Type {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            fn duplicate(&self) -> Box<dyn $crate::LogicContextData>
            where
                Self: Sized,
            {
                Box::new(self.clone())
            }
        }
    };
}
