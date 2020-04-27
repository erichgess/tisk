use clap::{App, Arg};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let args = App::new("Tisk")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Task Management with scoping")
        .subcommand(
            App::new("add")
            .arg(
                Arg::with_name("input")
                .index(1)
                .required(true)))
        .subcommand(
            App::new("close")
            .arg(
                Arg::with_name("ID")
                .index(1)
            )
        )
        .get_matches();

    if let Some(ref matches) = args.subcommand_matches("add") {
        let name =  matches.value_of("input").unwrap();
        let t = Task{
            id: 1,
            name: String::from(name),
            status: Status::Open,
        };

        match Task::write(&t, "1.yaml") {
            Ok(_) => (),
            Err(why) => panic!(why),
        }
    } else if let Some(ref done) = args.subcommand_matches("close") {
        let id = done.value_of("ID").unwrap();
        let path = format!("{}.yaml", id);
        match Task::read(path.as_str()) {
            Ok(mut t) => {
                t.status = Status::Closed;
                Task::write(&t, path.as_str()).unwrap();
            },
            Err(why) => panic!(why),
        }
        println!("{}", id);
    } else {
        println!("list!");
        get_files(".");
    }
}

fn get_files(path: &str) -> std::io::Result<()> {
    use std::fs;

    let contents = fs::read_dir(path)?;
    let yaml_files = contents.filter(|f| f.as_ref().unwrap().path().extension().map(|e| e == "yaml").unwrap_or(false));
    for yaml in yaml_files {
        let file = yaml?;
        let task = Task::read(file.path().to_str().unwrap());
        println!("{:?}", task);
    }

    Ok(())
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
