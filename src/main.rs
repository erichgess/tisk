mod io;
mod table;
mod tasks;

use clap::{App, Arg, ArgMatches};
use log::{debug, info, LevelFilter};
use log4rs;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
};
use table::{TableFormatter, TableRow};
use tasks::{Task, TaskList};

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
    CheckoutTask(u32),
    CheckinTask,
}

type Effects = Vec<CommandEffect>;

fn main() {
    configure_logger();

    let args = configure_cli().get_matches();

    std::process::exit(match run(&args) {
        Ok(_) => 0,
        Err(err) => {
            let preface = console::style("Error").red();

            eprintln!("{}: {}", preface, err);
            1
        }
    });
}

fn run(args: &ArgMatches) -> Result<(), String> {
    if args.subcommand_matches("init").is_some() {
        match io::initialize() {
            Ok(io::InitResult::Initialized) => Ok(println!("Initialized directory")),
            Ok(io::InitResult::AlreadyInitialized) => Ok(println!("Already initialized")),
            Err(why) => ferror!("Failed to initialize tisk project: {}", why),
        }
    } else {
        match io::find_task_dir() {
            Err(why) => Err(why),
            Ok(task_path) => {
                match TaskList::read_tasks(&task_path) {
                    Err(why) => ferror!("Failed to read tasks: {}", why),
                    Ok(mut tasks) => {
                        // TODO: This was an experiment to look at the idea of decoupling the
                        // application of a command to the in memory data and the act of then
                        // writing any changes to disk.  Now that the implementation is more or
                        // less done, think about if the design actually works.  My hypothesis
                        // was that doing this decoupling would make it harder to fail to write
                        // changed data to disk.
                        //
                        // 1. Does it make it easer to reason about the code.  What this design
                        //    does  do is explicitly show the user what effects they can have.
                        // 2. Does it make the code safer or more robust
                        // 3. What risks does this design bring: the changes you make and their
                        //    being committed are decoupled and far away, so it is hard to reason
                        //    about them.
                        //    - Having multiple effects which you want to make dependent would not
                        //    work in the present design; what could be done is some kind of
                        //    chaining where you have an effect and then something that is executed
                        //    if the effect is successfully resolved (e.g. `(Effect::Write,
                        //    and_then: () -> Effect)`

                        // load checked out task, if one is checked out
                        let checked_out_task =
                            io::read_checkout(&task_path).or_else(|err| ferror!("{}", err))?;

                        // Apply the given command to the in memory TaskList
                        let effects = execute_command(&mut tasks, checked_out_task, &args)?;
                        effects
                            .into_iter()
                            .map(|effect| match effect {
                                CommandEffect::Read => Ok(()),
                                CommandEffect::Write => {
                                    debug!("Writing tasks");
                                    tasks
                                        .write_all(&task_path)
                                        .or_else(|err| ferror!("{}", err))
                                }
                                CommandEffect::CheckoutTask(id) => {
                                    debug!("Checkout task {}", id);
                                    io::commit_checkout(id, &task_path)
                                        .or_else(|err| ferror!("{}", err))
                                }
                                CommandEffect::CheckinTask => {
                                    debug!("Checkin task");
                                    io::commit_checkin(&task_path).or_else(|err| ferror!("{}", err))
                                }
                            })
                            .collect()
                    }
                }
            }
        }
    }
}

// TODO: I kind of feel like passing this &mut TaskList into this function breaks the concept of
// the owner determining who can modify an entity
fn execute_command(
    tasks: &mut TaskList,
    checked_out_task: Option<u32>,
    args: &ArgMatches,
) -> Result<Effects, String> {
    match args.subcommand() {
        ("add", Some(args)) => handle_add(tasks, args),
        ("close", Some(args)) => handle_close(tasks, checked_out_task, args),
        ("edit", Some(args)) => handle_edit(tasks, checked_out_task, args),
        ("note", Some(args)) => handle_note(tasks, checked_out_task, args),
        ("checkout", Some(args)) => handle_checkout(tasks, args),
        ("checkin", Some(_)) => handle_checkin(),
        ("list", Some(args)) => handle_list(tasks, checked_out_task, args),
        _ => handle_list(tasks, checked_out_task, &ArgMatches::new()),
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
                )
                .arg(
                    Arg::with_name("note")
                        .long("note")
                        .short("n")
                        .takes_value(true)
                        .help("Adds a note to the newly created task."),
                ),
        )
        .subcommand(
            App::new("close")
                .about("Close a given task")
                .arg(Arg::with_name("ID").index(1))
                .arg(
                    Arg::with_name("note")
                        .long("note")
                        .short("n")
                        .takes_value(true)
                        .help("Adds a note to the newly created task."),
                ),
        )
        .subcommand(
            App::new("checkout")
                .about("Checkout a task.  This will cause task specific actions to apply to the checked out task if an ID is not provided.")
                .arg(Arg::with_name("ID").index(1))
                .arg(
                    Arg::with_name("add")
                        .long("add")
                        .short("a")
                        .takes_value(true)
                        .help("Adds a new task and immediately checks it out"),
                )
        )
        .subcommand(
            App::new("checkin")
                .about("Releases the currently checked out task. No task will be checked out afterwards.")
        )
        .subcommand(
            App::new("edit")
                .about("Change properties for an existing task")
                .arg(Arg::with_name("ID").index(1))
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
                .about("Add a note to a specific task.  Will attempt to add a note to the checked out task, unless the 'id' flag is used")
                .arg(Arg::with_name("NOTE").index(1))
                .arg(Arg::with_name("ID").long("id").help("Specify the Task ID, this overrides the checked out task and is required if no task is checked out"))
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

fn handle_add(tasks: &mut TaskList, args: &ArgMatches) -> Result<Effects, String> {
    let name = args.value_of("input").unwrap();
    let priority: u32 = args.value_of("priority").unwrap_or("1").parse().unwrap();

    debug!("Adding new task to task list");
    let id = tasks.add_task(name, priority);

    let note = args.value_of("note");
    note.and_then(|n| tasks.get_mut(id).map(|t| t.add_note(n)));
    Ok(vec![CommandEffect::Write])
}

fn handle_close(tasks: &mut TaskList, checked_out_task: Option<u32>, args: &ArgMatches) -> Result<Effects, String> {
    info!("{:?}", checked_out_task);
    let id = parse_integer_arg(args.value_of("ID"))
        .or_else(|e| ferror!("{}", e))?
        .or_else(|| checked_out_task)
        .ok_or("No ID provided and no task checked out")
        .or_else(|why| ferror!("{}", why))?;  // TODO: this is gnarly: probably should not do formatting at this level but at the level where the message is being printed.

    debug!("Closing task with ID: {}", id);
    match args.value_of("note") {
        Some(note) => tasks.get_mut(id).iter_mut().for_each(|t| t.add_note(note)),
        None => (),
    }
    match tasks.close_task(id) {
        None => ferror!("Could not find task with ID {}", id),
        Some(t) => {
            println!("Task {} was closed", t.id());
            Ok(vec![CommandEffect::Write])
        }
    }
}

fn handle_checkout(tasks: &mut TaskList, args: &ArgMatches) -> Result<Effects, String> {
    let mut effects = vec![];
    if args.is_present("ID") && args.is_present("add") {
        return ferror!("Cannot have an ID and the --add flag set at the same time");
    } else if !args.is_present("ID") && !args.is_present("add") {
        return ferror!("Must specify either an ID to checkout or `--add` to add a new task");
    }

    let id = match args.value_of("add") {
        Some(task) => {
            effects.push(CommandEffect::Write);
            tasks.add_task(task, 1)
        }
        None => match parse_integer_arg(args.value_of("ID")) {
            Err(_) => {
                return ferror!(
                    "Invalid ID provided, must be an integer greater than or equal to 0"
                )
            }
            Ok(None) => return ferror!("No ID provided"),
            Ok(Some(id)) => id,
        },
    };

    match tasks.get(id) {
        None => ferror!("Could not find task with ID {}", id),
        Some(_) => {
            debug!("Checkout task {}", id);
            println!("Checkout task {}", id);
            effects.push(CommandEffect::CheckoutTask(id));
            Ok(effects)
        }
    }
}

fn handle_checkin() -> Result<Effects, String> {
    // Generate a signal to delete the checkout file
    Ok(vec![CommandEffect::CheckinTask])
}

fn handle_edit(
    tasks: &mut TaskList,
    checked_out_task: Option<u32>,
    args: &ArgMatches,
) -> Result<Effects, String> {
    debug!("{:?}", checked_out_task);
    let id = parse_integer_arg(args.value_of("ID"))
        .or_else(|e| ferror!("{}", e))?
        .or_else(|| checked_out_task)
        .ok_or("No ID provided and no task checked out")
        .or_else(|why| ferror!("{}", why))?;  // TODO: this is gnarly: probably should not do formatting at this level but at the level where the message is being printed.

    let priority = match parse_integer_arg(args.value_of("priority")) {
        Err(_) => {
            return ferror!("Invalid priority value: must be an integer greater than or equal to 0")
        }
        Ok(p) => p,
    };

    match priority {
        None => Ok(vec![CommandEffect::Read]),
        Some(p) => match tasks.set_priority(id, p) {
            None => ferror!("Could not find task with ID {}", id),
            Some((old, new)) => {
                println!(
                    "Task {} priority set from {} to {}",
                    id,
                    old.priority(),
                    new.priority()
                );
                Ok(vec![CommandEffect::Write])
            }
        },
    }
}

fn handle_note(
    tasks: &mut TaskList,
    checked_out_task: Option<u32>,
    args: &ArgMatches,
) -> Result<Effects, String> {
    let id = match args.value_of("ID") {
        Some(id) => match parse_integer_arg(Some(id)).or_else(|err| Err(format!("{}", err)))? {
            Some(id) => id,
            None => return ferror!("Must have a task checked out or provide an id"),
        },
        None => match checked_out_task {
            Some(id) => id,
            None => return ferror!("Must have a task checked out or provide an id"),
        },
    };

    if args.is_present("list") || !args.is_present("NOTE") {
        let notes = tasks
            .get(id)
            .ok_or(format!("Could not found task with ID {}", id))?
            .notes();
        print_notes(notes);

        Ok(vec![CommandEffect::Read])
    } else {
        let note = match args.value_of("NOTE") {
            None => {
                return ferror!(
                    "Must provide a note to add. Or use the --list flag to list notes on this task"
                )
            }
            Some(note) => note,
        };

        match tasks.get_mut(id) {
            Some(task) => {
                task.add_note(note);
                Ok(vec![CommandEffect::Write])
            }
            None => ferror!("No task with id {} found.", id),
        }
    }
}

fn handle_list(
    tasks: &TaskList,
    checked_out_task: Option<u32>,
    args: &ArgMatches,
) -> Result<Effects, String> {
    if args.is_present("all") {
        let mut task_slice = tasks.get_all();
        task_slice.sort_by(|a, b| order_tasks(&b, &a));
        print_task_list(task_slice, checked_out_task);
    } else if args.is_present("closed") {
        let mut task_slice = tasks.get_closed();
        task_slice.sort_by(|a, b| order_tasks(&b, &a));
        print_task_list(task_slice, checked_out_task);
    } else {
        let mut task_slice = tasks.get_open();
        task_slice.sort_by(|a, b| order_tasks(&b, &a));
        print_task_list(task_slice, checked_out_task);
    }
    Ok(vec![CommandEffect::Read])
}

fn order_tasks(a: &Task, b: &Task) -> std::cmp::Ordering {
    let priority_cmp = a.priority().cmp(&b.priority());
    if priority_cmp == std::cmp::Ordering::Equal {
        b.created_at().cmp(&a.created_at())
    } else {
        priority_cmp
    }
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

pub fn print_task_list(tasks: Vec<&tasks::Task>, checked_out_task: Option<u32>) {
    use console::Term;

    // Get terminal dimensions so that we can compute how wide columns can be and
    // how to format text properly
    // Assume that we'll always have at least 20 columns in the terminal (as even that small
    // would be unuseable for a person.
    let (_, cols) = Term::stdout()
        .size_checked()
        .expect("Could not get terminal details");

    let id_width: usize = 4;
    let date_width: usize = 10; // YYYY-mm-dd
    let priority_width: usize = 3;
    let notes_width = 3;

    let mut tf = TableFormatter::new(cols as usize);
    tf.set_columns(vec![
        ("ID", Some(id_width)),
        ("Date", Some(date_width)),
        ("Name", None),
        ("Pri", Some(priority_width)),
        ("Nts", Some(notes_width)),
    ]);

    // Print the table
    let checkout_style = console::Style::new().green();
    let default_style = console::Style::new().white();
    tf.print_header();
    for task in tasks.iter() {
        let mut row = TableRow::new();
        row.push(task.id());
        row.push(task.created_at().format("%Y-%m-%d"));
        row.push(task.name());
        row.push(task.priority());
        row.push(task.notes().len());

        let print_row = match (checked_out_task, tf.print_row(row)) {
            (Some(id), row) if id == task.id() => checkout_style.apply_to(row),
            (_, row) => default_style.apply_to(row),
        };

        print!("{}", print_row);
    }
}

pub fn print_notes(notes: Vec<&tasks::Note>) {
    use console::Term;

    // Get terminal dimensions so that we can compute how wide columns can be and
    // how to format text properly
    // Assume that we'll always have at least 20 columns in the terminal (as even that small
    // would be unuseable for a person.
    let (_, cols) = Term::stdout()
        .size_checked()
        .expect("Could not get terminal details");

    let id_width: usize = 4;

    // Print the column headers
    let mut tf = TableFormatter::new(cols as usize);
    tf.set_columns(vec![("ID", Some(id_width)), ("Note", None)]);
    tf.print_header();

    // print each task, in the order given by the input vector
    let mut idx = 1;
    for note in notes.iter() {
        //Note::print_note(task, idx, id_width, note_width);
        let mut row = TableRow::new();
        row.push(idx);
        row.push(note.note());
        print!("{}", tf.print_row(row));
        idx += 1;
    }
}
