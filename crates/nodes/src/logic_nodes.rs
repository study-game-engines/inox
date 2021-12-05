use sabi_serialize::{deserialize, Deserialize, Serialize};

use crate::{
    implement_node, implement_pin, LogicData, Node, NodeState, NodeTrait, NodeTree, PinId,
};
use sabi_serialize::typetag;

#[derive(Serialize, Deserialize, Copy, Clone)]
#[serde(crate = "sabi_serialize")]
pub enum LogicExecution {
    Type,
}
impl Default for LogicExecution {
    fn default() -> Self {
        LogicExecution::Type
    }
}
implement_pin!(LogicExecution);

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "sabi_serialize")]
pub struct RustExampleNode {
    node: Node,
}
implement_node!(RustExampleNode, node, "Example", "Rust example node");
impl Default for RustExampleNode {
    fn default() -> Self {
        let mut node = Node::new(stringify!(RustExampleNode));
        node.add_input("in_int", 0_i32);
        node.add_input("in_float", 0_f32);
        node.add_input("in_string", String::new());
        node.add_input("in_bool", false);
        node.add_input("in_execute", LogicExecution::default());

        node.add_output("out_execute", LogicExecution::default());
        node.add_output("out_int", 0_i32);
        node.add_output("out_float", 0_f32);
        node.add_output("out_string", String::new());
        node.add_output("out_bool", false);
        Self { node }
    }
}
impl RustExampleNode {
    pub fn on_update(&mut self) -> NodeState {
        println!("Executing {}", self.name());
        println!("in_int {}", self.node().get_input::<i32>("in_int").unwrap());
        println!(
            "in_float {}",
            self.node().get_input::<f32>("in_float").unwrap()
        );
        println!(
            "in_string {}",
            self.node().get_input::<String>("in_string").unwrap()
        );
        println!(
            "in_bool {}",
            self.node().get_input::<bool>("in_bool").unwrap()
        );

        self.node_mut().pass_value::<i32>("in_int", "out_int");
        self.node_mut().pass_value::<f32>("in_float", "out_float");
        self.node_mut()
            .pass_value::<String>("in_string", "out_string");
        self.node_mut().pass_value::<bool>("in_bool", "out_bool");
        NodeState::Executed(vec![PinId::new("out_execute")])
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "sabi_serialize")]
pub struct ScriptInitNode {
    node: Node,
}
implement_node!(ScriptInitNode, node, "Init", "Script init node");
impl Default for ScriptInitNode {
    fn default() -> Self {
        let mut node = Node::new(stringify!(ScriptInitNode));
        node.add_output("execute", LogicExecution::default());
        Self { node }
    }
}
impl ScriptInitNode {
    pub fn on_update(&mut self) -> NodeState {
        println!("Executing {}", self.name());
        NodeState::Executed(vec![PinId::new("execute")])
    }
}

#[allow(dead_code)]
fn test_node() {
    use crate::LogicNodeRegistry;
    use sabi_serialize::serialize;

    let mut registry = LogicNodeRegistry::default();
    registry.register_node::<ScriptInitNode>();
    registry.register_node::<RustExampleNode>();

    registry.register_pin_type::<f32>();
    registry.register_pin_type::<f64>();
    registry.register_pin_type::<u8>();
    registry.register_pin_type::<i8>();
    registry.register_pin_type::<u16>();
    registry.register_pin_type::<i16>();
    registry.register_pin_type::<u32>();
    registry.register_pin_type::<i32>();
    registry.register_pin_type::<bool>();
    registry.register_pin_type::<String>();
    registry.register_pin_type::<LogicExecution>();

    let mut tree = NodeTree::default();
    tree.add_link("ScriptInitNode", "NodeA", "execute", "in_execute");
    tree.add_link("NodeA", "NodeB", "out_int", "in_int");
    tree.add_link("NodeA", "NodeB", "out_string", "in_string");
    tree.add_link("NodeA", "NodeB", "out_execute", "in_execute");
    assert_eq!(tree.get_links_count(), 4);

    let init = ScriptInitNode::default();
    let serialized_data = init.serialize_node();

    if let Some(n) = registry.deserialize_node(&serialized_data) {
        tree.add_node(n);
    }
    assert_eq!(tree.get_nodes_count(), 1);

    let mut node_a = RustExampleNode::default();
    node_a.set_name("NodeA");
    if let Some(v) = node_a.node_mut().get_input_mut::<i32>("in_int") {
        *v = 19;
    }
    if let Some(v) = node_a.node_mut().get_input_mut::<f32>("in_float") {
        *v = 22.;
    }
    if let Some(v) = node_a.node_mut().get_input_mut::<String>("in_string") {
        *v = String::from("Ciao");
    }
    if let Some(v) = node_a.node_mut().get_input_mut::<bool>("in_bool") {
        *v = true;
    }
    assert_eq!(*node_a.node().get_input::<i32>("in_int").unwrap(), 19);
    assert_eq!(*node_a.node().get_output::<i32>("out_int").unwrap(), 0);
    assert_eq!(*node_a.node().get_input::<f32>("in_float").unwrap(), 22.);
    assert_eq!(*node_a.node().get_output::<f32>("out_float").unwrap(), 0.);
    assert_eq!(
        *node_a.node().get_input::<String>("in_string").unwrap(),
        String::from("Ciao")
    );
    assert_eq!(
        *node_a.node().get_output::<String>("out_string").unwrap(),
        String::new()
    );
    assert!(*node_a.node().get_input::<bool>("in_bool").unwrap());
    assert!(!*node_a.node().get_output::<bool>("out_bool").unwrap());
    let serialized_data = node_a.serialize_node();

    if let Some(n) = registry.deserialize_node(&serialized_data) {
        tree.add_node(n);
    }
    assert_eq!(tree.get_nodes_count(), 2);

    tree.add_default_node::<RustExampleNode>("NodeB");
    assert_eq!(tree.get_nodes_count(), 3);

    let serialized_tree = serialize(&tree);
    if let Ok(new_tree) = deserialize::<NodeTree>(&serialized_tree) {
        let mut logic_data = LogicData::from(new_tree);
        assert_eq!(tree.get_nodes_count(), logic_data.tree().get_nodes_count());
        logic_data.init();
        logic_data.execute();
    } else {
        panic!("Deserialization failed");
    }
}

#[test]
fn test_node_fn() {
    test_node()
}