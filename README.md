# Tisk: Project Task Management
Tisk manages tasks which are localized to a project or repository, rather than
globally.  Similar to git, tasks are locallized to a project directory and all
of its subdirectories; so the tasks you are shown are always the ones relevant
to what you are currently working on.

Tasks are also easily used for keeping notes about work, so that when working
on a task you can easily record thoughts, notes, and reminders.

## Usage
### Installation
Installation is done with `cargo`:

```
cargo install --path .
```

This will build the `tisk` project and then copy the binary to your local `cargo`
installation bin directory.  As long as that path is in your `PATH` env variable
then `tisk` will be accessible from the command line.

### Initializing a Tisk Project
Like git, the first thing that must be done to track tasks for a project is
to initialize it as a Tisk project.  Run `tisk init` in the project's root
diretory.  This will create the `.tisk/` directory which is used to store
all the task data for this project.  All subdirectories will use this `.tisk`
for managing tasks.

### Tasks
1. `tisk add <TASK>` - this will add a new task to the project.
2. `tisk` - running with no subcommands or options will print a list of all
the open tasks for the project, ordered by priority.
3. `tisk checkout <ID>` - will make the task with id `ID` the current task
which will cause any command which takes an `ID` to use the checked out
task if no `ID` is given.
4. `tisk checkin` - sets no task as checked out.
5. `tisk close <ID>` - will close `ID` task.  If no `ID` is given it will
close the checked out task.
6. `tisk note <ID> <NOTE>` - will add a note to the task with id `ID`,
if no `ID` is given it will use the checked out task.
7. `tisk note <ID>` - will print out a list of all the notes associated
with the given task, if not `ID` is given then it will use the checked out 
task.
