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
        .subcommand(
            App::new("init")
        )
        .get_matches();

    if args.subcommand_matches("init").is_some() {
        match initialize() {
            Ok(InitResult::Initialized) => println!("Initialized directory"),
            Ok(InitResult::AlreadyInitialized) => println!("Already initialized"),
            Err(why) => panic!("Failed to initialize directory: {}", why),
        }
    } else 
    {
        let task_path = match get_task_path() {
            Some(path) => path,
            None => panic!("Not a task project: no .task directory found in this directory or a parent dirctory"),
        };

        let mut tasks = match get_files(task_path) {
            Err(why) => panic!("Failed to get YAML files: {}", why),
            Ok(files) => {
                TaskList::read_tasks(files).unwrap()
            }
        };

        if let Some(ref matches) = args.subcommand_matches("add") {
            let name =  matches.value_of("input").unwrap();
            let t = Task{
                id: tasks.next_id(),
                name: String::from(name),
                status: Status::Open,
            };

            let path = std::path::PathBuf::from(format!("{}/{}.yaml", task_path, t.id));
            match Task::write(&t, &path) {
                Ok(_) => (),
                Err(why) => panic!(why),
            }
        } else if let Some(ref done) = args.subcommand_matches("close") {
            let id: u32 = done.value_of("ID").unwrap().parse().unwrap();
            match tasks.get_mut(id) {
                None => println!("No task with ID {} found", id),
                Some(mut task) => {
                    task.status = Status::Closed;
                    let path = std::path::PathBuf::from(format!("{}/{}.yaml", task_path, id));
                    Task::write(&task, &path).unwrap();
                },
            }
        } else {
            for task in tasks.tasks {
                println!("{:?}", task);
            }
        }
    }
}

fn get_files(path: &str) -> std::io::Result<Vec<std::path::PathBuf>> {
    use std::fs;

    let contents = fs::read_dir(path)?;
    let yaml_files = contents.filter(|f| f.as_ref().unwrap().path().extension().map(|e| e == "yaml").unwrap_or(false));
    let mut files = vec![];
    for yaml in yaml_files {
        let file = yaml?;
        files.push(file.path());
    }

    Ok(files)
}

enum InitResult {
    Initialized,
    AlreadyInitialized,
}

fn initialize() -> std::io::Result<InitResult> {
        match std::fs::read_dir("./.task") {
            Ok(_) => Ok(InitResult::AlreadyInitialized),
            Err(_) => match std::fs::create_dir("./.task") {
                Err(why) => Err(why),
                Ok(_) => Ok(InitResult::Initialized),
            }
        }
}

fn get_task_path() -> Option<&'static str> {
    match std::fs::read_dir("./.task") {
        Ok(_) => Some("./.task"),
        Err(_) => None,
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
    fn write(task: &Task, path: &std::path::PathBuf) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        let s = serde_yaml::to_string(task).unwrap();

        file.write_all(s.as_bytes())
    }

    fn read(path: &std::path::PathBuf) -> std::io::Result<Task> {
        let mut file = File::open(path)?;

        let mut s = String::new();
        file.read_to_string(&mut s)?;

        let y = serde_yaml::from_str::<Task>(&s).unwrap();
        Ok(y)
    }

}

struct TaskList {
    tasks: Vec<Task>,
}

impl TaskList {
    fn read_tasks(paths: Vec<std::path::PathBuf>) -> std::io::Result<TaskList> {
        let mut tasks = vec![];
        for path in paths.into_iter() {
            let task = Task::read(&path)?;
            tasks.push(task);
        }
        Ok(TaskList{tasks})
    }

    fn next_id(&self) -> u32 {
        if self.tasks.len() == 0 {
            1
        } else {
            let mut largest_id = self.tasks[0].id;
            for task in self.tasks.iter() {
                if task.id > largest_id {
                    largest_id = task.id;
                }
            }
            largest_id+1
        }
    }

    fn get_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
}
