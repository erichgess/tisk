use clap::{App, Arg, ArgMatches};
use log::{debug, LevelFilter};
use log4rs;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
};
extern crate tisk;

/**
 * This indicates what effect executing  a command had on the task list.
 * `Read` means that the command only read from the task list and thus
 * no changes were made and nothing needs to be written.
 *
 * `Write` means that the command modified the TaskList or a Task in the
 * TaskList and the changes will need to be written to disk.
 */
enum CommandEffect {
    Write,
    Read,
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

fn main() {
    configure_logger();

    let args = configure_cli().get_matches();

    std::process::exit(match run(&args) {
        Ok(_) => 0,
        Err(err) => {
            println!("> {}", err);
            1
        }
    });
}

fn run(args: &ArgMatches) -> Result<(), String> {
    // TODO: I don't like how the if blocks break down into "init" in one very simple
    // block, then another big block that handles all other subcommands.  What I don't
    // like is that it's not obvious why that is (because init is a meta step which acts
    // on the setup of tisk rather than an operation on the task data).
    if args.subcommand_matches("init").is_some() {
        match tisk::initialize() {
            Ok(tisk::InitResult::Initialized) => Ok(println!("Initialized directory")),
            Ok(tisk::InitResult::AlreadyInitialized) => Ok(println!("Already initialized")),
            Err(why) => ferror!("Failed to initialize tisk project: {}", why),
        }
    } else {
        match find_task_dir() {
            Err(why) => Err(why),
            Ok(task_path) => {
                match tisk::TaskList::read_tasks(&task_path) {
                    Err(why) => ferror!("Failed to read tasks: {}", why),
                    Ok(mut tasks) => {
                        // TODO: This was an experiment to look at the idea of decoupling the
                        // application of a command to the in memory data and the act of then
                        // writing any changes to disk.  Now that the implementation is more or
                        // less done, think about if the design actually works.  My hypothesis
                        // was that doing this decoupling would make it harder to fail to write
                        // changed data to disk.
                        //
                        // 1. Does it make it easer to reason about the code
                        // 2. Does it make the code safer or more robust
                        // 3. What risks does this design bring

                        // Apply the given command to the in memory TaskList
                        let result = execute_command(&mut tasks, &args);

                        // Determine if the TaskList needs to be written to disk
                        match result {
                            Err(e) => Err(e),
                            Ok(CommandEffect::Read) => Ok(()),
                            Ok(CommandEffect::Write) => {
                                debug!("Writing tasks");
                                match tasks.write_all(&task_path) {
                                    Ok(_) => Ok(()),
                                    Err(why) => ferror!("{}", why),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// TODO: I kind of feel like passing this &mut TaskList into this function breaks the concept of
// the owner determining who can modify an entity
fn execute_command(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<CommandEffect, String> {
    if let Some(add) = args.subcommand_matches("add") {
        handle_add(tasks, add)
    } else if let Some(close) = args.subcommand_matches("close") {
        handle_close(tasks, close)
    } else if let Some(edit) = args.subcommand_matches("edit") {
        handle_edit(tasks, edit)
    } else if let Some(note) = args.subcommand_matches("note") {
        handle_note(tasks, note)
    } else {
        if let Some(list) = args.subcommand_matches("list") {
            handle_list(tasks, list)
        } else {
            handle_list(tasks, &ArgMatches::new())
        }
    }
}

fn find_task_dir() -> Result<std::path::PathBuf, String> {
    match tisk::up_search(".", ".tisk") {
        Err(why) => ferror!("Failure while searching for .tisk dir: {}", why),
        Ok(path) => match path {
            None => ferror!("Invalid tisk project, could not find .tisk dir in the current directory or any parent directory"),
            Some(path) => Ok(path),
        }
    }
}

fn configure_cli<'a, 'b>() -> App<'a, 'b> {
    App::new("Tisk")
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
            App::new("note")
                .about("Add a note to a specific task")
                .arg(Arg::with_name("ID").index(1).required(true))
                .arg(Arg::with_name("NOTE").index(2))
                .arg(Arg::with_name("list").long("list").short("l")),
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
}

fn handle_add(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<CommandEffect, String> {
    let name = args.value_of("input").unwrap();
    let priority: u32 = args.value_of("priority").unwrap_or("1").parse().unwrap();

    debug!("Adding new task to task list");
    tasks.add_task(name, priority);
    Ok(CommandEffect::Write)
}

fn handle_close(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<CommandEffect, String> {
    let id = match parse_integer_arg(args.value_of("ID")) {
        Err(_) => {
            return ferror!("Invalid ID provided, must be an integer greater than or equal to 0")
        }
        Ok(None) => return ferror!("No ID provided"),
        Ok(Some(id)) => id,
    };

    debug!("Closing task with ID: {}", id);
    match tasks.close_task(id) {
        None => ferror!("Could not find task with ID {}", id),
        Some(t) => {
            println!("Task {} was closed", t.id());
            Ok(CommandEffect::Write)
        }
    }
}

fn handle_edit(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<CommandEffect, String> {
    let id = match parse_integer_arg(args.value_of("ID")) {
        Err(_) => {
            return ferror!("Invalid ID provided, must be an integer greater than or equal to 0")
        }
        Ok(None) => return ferror!("No ID provided"),
        Ok(Some(id)) => id,
    };

    let priority = match parse_integer_arg(args.value_of("priority")) {
        Err(_) => {
            return ferror!("Invalid priority value: must be an integer greater than or equal to 0")
        }
        Ok(p) => p,
    };

    match priority {
        None => Ok(CommandEffect::Read),
        Some(p) => match tasks.set_priority(id, p) {
            None => ferror!("Could not find task with ID {}", id),
            Some((old, new)) => {
                println!(
                    "Task {} priority set from {} to {}",
                    id,
                    old.priority(),
                    new.priority()
                );
                Ok(CommandEffect::Write)
            }
        },
    }
}

fn handle_note(tasks: &mut tisk::TaskList, args: &ArgMatches) -> Result<CommandEffect, String> {
    let id = match parse_integer_arg(args.value_of("ID")) {
        Err(_) => {
            return ferror!("Invalid ID provided, must be an integer greater than or equal to 0")
        }
        Ok(None) => return ferror!("No ID provided"),
        Ok(Some(id)) => id,
    };

    if args.is_present("list") {
        let notes = tasks
            .get(id)
            .ok_or(format!("Could not found task with ID {}", id))?
            .notes();
        for note in notes {
            println!("{}", note.note());
        }
        Ok(CommandEffect::Read)
    } else {
        let note = match args.value_of("NOTE") {
            None => {
                return ferror!(
                    "Invalid note provided, must be an integer greater than or equal to 0"
                )
            }
            Some(note) => note,
        };

        match tasks.get_mut(id) {
            Some(task) => {
                task.add_note(note);
                Ok(CommandEffect::Write)
            }
            None => ferror!("No task with id {} found.", id),
        }
    }
}

fn handle_list(tasks: &tisk::TaskList, args: &ArgMatches) -> Result<CommandEffect, String> {
    if args.is_present("all") {
        let mut task_slice = tasks.get_all();
        task_slice.sort_by(|a, b| b.priority().cmp(&a.priority()));
        tisk::TaskList::print(task_slice);
    } else if args.is_present("closed") {
        let mut task_slice = tasks.get_closed();
        task_slice.sort_by(|a, b| b.priority().cmp(&a.priority()));
        tisk::TaskList::print(task_slice);
    } else {
        let mut task_slice = tasks.get_open();
        task_slice.sort_by(|a, b| b.priority().cmp(&a.priority()));
        tisk::TaskList::print(task_slice);
    }
    Ok(CommandEffect::Read)
}

fn parse_integer_arg(arg: Option<&str>) -> Result<Option<u32>, std::num::ParseIntError> {
    match arg {
        None => Ok(None),
        Some(v) => v.parse().map(|p| Some(p)),
    }
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
