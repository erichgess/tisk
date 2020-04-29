use clap::{App, Arg};
use log::{debug, LevelFilter};
use log4rs;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
};
extern crate tisk;

fn configure_logger() {
    match log4rs::init_file("config/log4rs.yaml", Default::default()) {
        Err(_) => {
            let stdout = ConsoleAppender::builder().build();
            let config = log4rs::config::Config::builder()
                .appender(Appender::builder().build("stdout", Box::new(stdout)))
                .build(Root::builder().appender("stdout").build(LevelFilter::Info))
                .unwrap();
            log4rs::init_config(config).unwrap();
        }
        Ok(_) => (),
    }
}

fn main() {
    configure_logger();

    let args = App::new("Tisk")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Task Management with scoping")
        .subcommand(App::new("add").arg(Arg::with_name("input").index(1).required(true)))
        .subcommand(App::new("close").arg(Arg::with_name("ID").index(1)))
        .subcommand(
            App::new("list").arg(
                Arg::with_name("all")
                    .help("Display all tasks, regardless of state")
                    .long("all"),
            ).arg(
                Arg::with_name("closed")
                    .help("Display all closed tasks")
                    .long("closed"),
            ).arg(
                Arg::with_name("open")
                    .help("Display all open tasks")
                    .long("open"),
            ),
        )
        .subcommand(App::new("init"))
        .get_matches();

    if args.subcommand_matches("init").is_some() {
        match tisk::initialize() {
            Ok(tisk::InitResult::Initialized) => println!("Initialized directory"),
            Ok(tisk::InitResult::AlreadyInitialized) => println!("Already initialized"),
            Err(why) => panic!("Failed to initialize tisk project: {}", why),
        }
    } else {
        let task_path = match tisk::up_search(".", ".tisk") {
            Ok(path) => match path {
                Some(p) => p,
                None => panic!("Invalid tisk project, could not found .tisk in this directory or any parent directory"),
            },
            Err(why) => panic!("Failure while searching for .tisk dir: {}", why),
        };

        let mut tasks = match tisk::TaskList::read_tasks(&task_path) {
            Err(why) => panic!("Failed to read tasks: {}", why),
            Ok(tasks) => tasks,
        };

        if let Some(ref matches) = args.subcommand_matches("add") {
            debug!("Adding new task to task list");
            let name = matches.value_of("input").unwrap();
            tasks.add_task(name);
        } else if let Some(ref done) = args.subcommand_matches("close") {
            let id: u32 = done.value_of("ID").unwrap().parse().unwrap();
            debug!("Closing task with ID: {}", id);
            tasks.close_task(id).expect("Could not find given ID");
        } else {
            if let Some(ref matches) = args.subcommand_matches("list") {
                if matches.is_present("all") {
                    let mut task_slice = tasks.get_all();
                    task_slice.sort_by(|a,b| a.id().cmp(&b.id()));
                    tisk::TaskList::print(task_slice);
                }  else if matches.is_present("closed") {
                    let mut task_slice = tasks.get_closed();
                    task_slice.sort_by(|a,b| a.id().cmp(&b.id()));
                    tisk::TaskList::print(task_slice);
                } else {
                    let mut task_slice = tasks.get_open();
                    task_slice.sort_by(|a,b| a.id().cmp(&b.id()));
                    tisk::TaskList::print(task_slice);
                }
            } else {
                let mut task_slice = tasks.get_open();
                task_slice.sort_by(|a,b| a.id().cmp(&b.id()));
                tisk::TaskList::print(task_slice);
            }
        }
        debug!("Writing tasks");
        match tasks.write_all(&task_path) {
            Ok(_) => (),
            Err(why) => panic!("Failed to write tasks: {}", why),
        }
    }
}
