use log4rs;
use log::{debug};
use clap::{App, Arg};
extern crate tisk;

fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default())
        .expect("Failed to configure logger");

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
        match tisk::initialize() {
            Ok(tisk::InitResult::Initialized) => println!("Initialized directory"),
            Ok(tisk::InitResult::AlreadyInitialized) => println!("Already initialized"),
            Err(why) => panic!("Failed to initialize tisk project: {}", why),
        }
    } else {
        let task_path = match tisk::up_search(".", ".task") {
            Ok(path) => match path {
                Some(p) => p,
                None => panic!("Invalid tisk project, could not found .task in this directory or any parent directory"),
            },
            Err(why) => panic!("Failure while searching for .task dir: {}", why),
        };

        let mut tasks = match tisk::get_files(&task_path) { //TODO: this should be owned by TaskList
            Err(why) => panic!("Failed to get YAML files: {}", why),
            Ok(files) => {
                tisk::TaskList::read_tasks(files).unwrap()
            }
        };

        if let Some(ref matches) = args.subcommand_matches("add") {
            debug!("Adding new task to task list");
            let name =  matches.value_of("input").unwrap();
            tasks.add_task(name).expect("Failed to create new task");
        } else if let Some(ref done) = args.subcommand_matches("close") {
            let id: u32 = done.value_of("ID").unwrap().parse().unwrap();
            debug!("Closing task with ID: {}", id);
            tasks.close_task(id).expect("Could not find given ID");
        } else {
            tasks.print();
        }
        debug!("Writing tasks");
        match tasks.write_all(&task_path) {
            Ok(_) => (),
            Err(why) => panic!("Failed to write tasks: {}", why),
        }
    }
}
