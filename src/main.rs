use clap::{App, Arg, ArgMatches};
use log::{debug, LevelFilter};
use log4rs;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
};
extern crate tisk;

//TODO: Could have this return a formatted string rather than print to stdout
macro_rules! error {
    () => {{
        use console::style;
        println!("{}: ", style("Error").red())
    }};
    ($($arg:tt)*) => {{
        use console::style;
        let preface = format!("{}: ", style("Error").red());
        print!("{}", preface);
        println!($($arg)*);
    }};
}

macro_rules! ferror {
    () => {{
        use console::style;
        Err(format!("{}: ", style("Error").red()))
    }};
    ($($arg:tt)*) => {{
        use console::style;
        let preface = format!("{}: ", style("Error").red());
        let msg = format!($($arg)*);
        Err(format!("{}{}", preface, msg))
    }};
}

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
        .subcommand(
            App::new("add")
                .about("Add a new task to the project")
                .arg(Arg::with_name("input").index(1).required(true))
                .arg(
                    Arg::with_name("priority")
                        .long("priority")
                        .short("p")
                        .takes_value(true)
                        .help("Sets the priority for this task (0+)."),
                ),
        )
        .subcommand(
            App::new("close")
                .about("Close a given task")
                .arg(Arg::with_name("ID").index(1)),
        )
        .subcommand(
            App::new("edit")
                .about("Change properties for an existing task")
                .arg(Arg::with_name("ID").index(1).required(true))
                .arg(
                    Arg::with_name("priority")
                        .long("priority")
                        .short("p")
                        .takes_value(true)
                        .help("Sets the priority for this task (0+)."),
                ),
        )
        .subcommand(
            App::new("list")
                .about("List the tasks in this project")
                .arg(
                    Arg::with_name("all")
                        .help("Display all tasks, regardless of state")
                        .long("all"),
                )
                .arg(
                    Arg::with_name("closed")
                        .help("Display all closed tasks")
                        .long("closed"),
                )
                .arg(
                    Arg::with_name("open")
                        .help("Display all open tasks")
                        .long("open"),
                ),
        )
        .subcommand(App::new("init").about("Intialize a new tisk project based in this directory"))
        .get_matches();

    if args.subcommand_matches("init").is_some() {
        match tisk::initialize() {
            Ok(tisk::InitResult::Initialized) => println!("Initialized directory"),
            Ok(tisk::InitResult::AlreadyInitialized) => println!("Already initialized"),
            Err(why) => error!("Failed to initialize tisk project: {}", why),
        }
    } else {
        let result: Result<(),String> = match tisk::up_search(".", ".tisk") {
            Err(why) => ferror!("Failure while searching for .tisk dir: {}", why),
            Ok(path) => match path {
                None => ferror!("Invalid tisk project, could not found .tisk in this directory or any parent directory"),
                Some(task_path) => {
                    match tisk::TaskList::read_tasks(&task_path) {
                        Err(why) => ferror!("Failed to read tasks: {}", why),
                        Ok(mut tasks) => {
                            let command_result = if let Some(add) = args.subcommand_matches("add") {
                                handle_add(&mut tasks, add)
                            } else if let Some(close) = args.subcommand_matches("close") {
                                handle_close(&mut tasks, close)
                            } else if let Some(edit) = args.subcommand_matches("edit") {
                                handle_edit(&mut tasks, edit)
                            } else {
                                if let Some(list) = args.subcommand_matches("list") {
                                    handle_list(&tasks, list)
                                } else {
                                    let mut task_slice = tasks.get_open();
                                    task_slice.sort_by(|a, b| b.priority().cmp(&a.priority()));
                                    Ok(tisk::TaskList::print(task_slice))
                                }
                            };
                            match command_result {
                                Err(e) => Err(e),
                                Ok(()) => {
                                    debug!("Writing tasks");
                                    match tasks.write_all(&task_path) {
                                        Ok(_) => Ok(()),
                                        Err(why) => ferror!("{}", why), //panic!("Failed to write tasks: {}", why),
                                    }
                                }
                            }
                        },
                    }
                },
            },
        };
        match result {
            Ok(_) => (),
            Err(err) => println!("{}", err),
        };
    }
}

fn handle_add(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<(), String> {
    let name = args.value_of("input").unwrap();
    let priority: u32 = args.value_of("priority").unwrap_or("1").parse().unwrap();

    debug!("Adding new task to task list");
    tasks.add_task(name, priority);
    Ok(())
}

fn handle_close(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<(), String> {
    let id = match parse_integer_arg(args.value_of("ID")) {
        Err(_) => ferror!("Invalid ID provided, must be an integer greater than or equal to 0"),
        Ok(None) => ferror!("No ID provided"),
        Ok(Some(id)) => Ok(id),
    }?;

    debug!("Closing task with ID: {}", id);
    match tasks.close_task(id) {
        None => ferror!("Could not find task with ID {}", id),
        Some(t) => Ok(println!("Task {} was closed", t.id())),
    }
}

fn handle_edit(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<(), String> {
    let id = parse_integer_arg(args.value_of("ID"));
    match id {
        Err(_) => ferror!("Invalid value given for ID, must be an integer."),
        Ok(None) => ferror!("Must provide a task ID"),
        Ok(Some(id)) => {
            let priority = parse_integer_arg(args.value_of("priority"));
            match priority {
                Err(_) => ferror!("Invalid value given for priority: must be an integer greater than or equal to 0."),
                Ok(p) => match p {
                    None => Ok(()),
                    Some(p) => match tasks.set_priority(id, p) {
                        None => ferror!("Could not find task with ID {}", id),
                        Some((old, new)) => Ok(println!("Task {} priority set from {} to {}", id, old.priority(), new.priority())),
                    },
                },
            }
        }
    }
}

fn handle_list(tasks: &tisk::TaskList, args: &ArgMatches) -> Result<(), String> {
    if args.is_present("all") {
        let mut task_slice = tasks.get_all();
        task_slice.sort_by(|a, b| b.priority().cmp(&a.priority()));
        tisk::TaskList::print(task_slice);
        Ok(())
    } else if args.is_present("closed") {
        let mut task_slice = tasks.get_closed();
        task_slice.sort_by(|a, b| b.priority().cmp(&a.priority()));
        tisk::TaskList::print(task_slice);
        Ok(())
    } else {
        let mut task_slice = tasks.get_open();
        task_slice.sort_by(|a, b| b.priority().cmp(&a.priority()));
        tisk::TaskList::print(task_slice);
        Ok(())
    }
}

fn parse_integer_arg(arg: Option<&str>) -> Result<Option<u32>, std::num::ParseIntError> {
    match arg {
        None => Ok(None),
        Some(v) => v.parse().map(|p| Some(p)),
    }
}
