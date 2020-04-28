use clap::{App, Arg};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashSet;

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
    } else {
        let mut task_path = match up_search(".task") {
            Ok(path) => match path {
                Some(p) => p,
                None => panic!("Invalid task project, could not found .task in this directory or any parent directory"),
            },
            Err(why) => panic!("Failure while searching for .task dir: {}", why),
        };

        let mut tasks = match get_files(&task_path) {
            Err(why) => panic!("Failed to get YAML files: {}", why),
            Ok(files) => {
                TaskList::read_tasks(files).unwrap()
            }
        };

        if let Some(ref matches) = args.subcommand_matches("add") {
            let name =  matches.value_of("input").unwrap();
            let t = tasks.add_task(name).unwrap();
            match Task::write(&t, &task_path) {
                Ok(_) => (),
                Err(why) => panic!(why),
            }
        } else if let Some(ref done) = args.subcommand_matches("close") {
            let id: u32 = done.value_of("ID").unwrap().parse().unwrap();
            match tasks.close_task(id) {
                None => println!("No task with ID {} found", id),
                Some(task) => {
                    Task::write(task, &task_path).unwrap();
                },
            }
        } else {
            for task in tasks.tasks {
                println!("{:?}", task);
            }
        }
    }
}

fn get_files(path: &std::path::PathBuf) -> std::io::Result<Vec<std::path::PathBuf>> {
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

fn up_search(file_name: &str) -> std::io::Result<Option<std::path::PathBuf>> {
    let path = std::fs::canonicalize(".")?;

    let mut found = None;

    for parent in path.ancestors() {
        let mut files = parent.read_dir()?;
        found = files.find(|f| {
            let file = f.as_ref().unwrap();
            let meta = file.metadata();
            match meta {
                Ok(md) => {
                    let ty = md.file_type();
                    if ty.is_dir() {
                        file.file_name() == file_name
                    } else {
                        false
                    }
                },
                Err(_) => false
            }
        });

        if found.is_some() {
            break;
        }
    }

    match found {
        Some(result) => match result {
            Err(why) => Err(why),
            Ok(v) => Ok(Some(v.path())),
        },
        None => Ok(None),
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
        let mut path = std::path::PathBuf::from(path);
        path.push(format!("{}.yaml", task.id));
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
    modified_tasks: HashSet<u32>,
}

impl TaskList {
    fn read_tasks(paths: Vec<std::path::PathBuf>) -> std::io::Result<TaskList> {
        let mut tasks = vec![];
        for path in paths.into_iter() {
            let task = Task::read(&path)?;
            tasks.push(task);
        }
        Ok(TaskList{
            tasks,
            modified_tasks: HashSet::new(),
        })
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

    /**
     * Searches the `TaskList` for a task with the given ID.  If a matching
     * task is found: then return a mutable reference to that task and mark
     * the task as modified.
     *
     * If no task is found with the given ID then return `None`.
     */
    fn get_mut(&mut self, id: u32) -> Option<&mut Task> {
        let task = self.tasks.iter_mut().find(|t| t.id == id);

        // Assume that any call to get a mutable reference to a task
        // will result in that task being modified.
        match &task {
            Some(task) => {
                self.modified_tasks.insert(task.id);
            },
            None => ()
        }
        task
    }

    fn get(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    fn add_task(&mut self, name: &str) -> Option<&Task> {
        let id = self.next_id();
        let t = Task{
            id: id,
            name: String::from(name),
            status: Status::Open,
        };
        self.tasks.push(t);
        self.get(id)
    }

    fn close_task(&mut self, id: u32) -> Option<&Task> {
        match self.get_mut(id) {
            None => None,
            Some(task) => {
                task.status = Status::Closed;
                Some(task)
            }
        }
    }

    fn write_all(self, task_path: &std::path::PathBuf) -> std::io::Result<u32> {
        let mut count = 0;
        for id in self.modified_tasks.iter() {
            match self.get(*id) {
                Some(task) => {
                    Task::write(task, &task_path)?;
                    count += 1;
                },
                None => (),
            }
        }
        Ok(count)
    }
}
