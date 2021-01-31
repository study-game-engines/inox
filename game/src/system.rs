use nrg_app::*;

pub struct MySystem {
    id: SystemId,
}

impl MySystem {
    pub fn new() -> Self {
        Self {
            id: SystemId::new(),
        }
    }
} 

impl System for MySystem {
    type In = ();
    type Out = ();

    fn id(&self) -> SystemId {
        self.id
    }    
    fn init(&mut self) {
        //println!("Executing init() for MySystem[{:?}]", self.id());
    }
    fn run(&mut self, _input: Self::In) -> Self::Out {
        //println!("Executing run() for MySystem[{:?}]", self.id());
    }
    fn uninit(&mut self) {
        //println!("Executing uninit() for MySystem[{:?}]", self.id());
    }
}
