use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let t = Task{
        id: 1,
        name: String::from("test"),
        status: Status::Open,
    };

    match Task::write(&t, "1.yaml") {
        Ok(_) => (),
        Err(why) => panic!(why),
    }

    match Task::read("1.yaml") {
        Ok(t) => println!("{:?}", t),
        Err(why) => panic!(why),
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Status {
    Open,
    Closed,
}

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: u32,
    name: String,
    status: Status,
}

impl Task {
    fn write(task: &Task, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        let s = serde_yaml::to_string(task).unwrap();

        file.write_all(s.as_bytes())
    }

    fn read(path: &str) -> std::io::Result<Task> {
        let mut file = File::open(path)?;

        let mut s = String::new();
        file.read_to_string(&mut s)?;

        let y = serde_yaml::from_str::<Task>(&s).unwrap();
        Ok(y)
    }
}
