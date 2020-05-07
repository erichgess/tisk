use super::io::get_files;
use super::task::{Status, Task};

pub struct TaskList {
    tasks: Vec<Task>,
}

impl TaskList {
    pub fn read_tasks(path: &std::path::PathBuf) -> std::io::Result<TaskList> {
        let paths = get_files(path)?;
        let mut tasks = vec![];
        for p in paths.into_iter() {
            let task = Task::read(&p)?;
            tasks.push(task);
        }
        Ok(TaskList { tasks })
    }

    pub fn next_id(&self) -> u32 {
        if self.tasks.len() == 0 {
            1
        } else {
            let mut largest_id = self.tasks[0].id();
            for task in self.tasks.iter() {
                if task.id() > largest_id {
                    largest_id = task.id();
                }
            }
            largest_id + 1
        }
    }

    /**
     * Searches the `TaskList` for a task with the given ID.  If a matching
     * task is found: then return a mutable reference to that task and mark
     * the task as modified.
     *
     * If no task is found with the given ID then return `None`.
     */
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id() == id)
    }

    pub fn get(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id() == id)
    }

    pub fn add_task(&mut self, name: &str, priority: u32) -> u32 {
        let id = self.next_id();
        let t = Task::new(id, String::from(name), Status::Open, priority);
        self.tasks.push(t);

        id
    }

    pub fn close_task(&mut self, id: u32) -> Option<&Task> {
        match self.get_mut(id) {
            None => None,
            Some(task) => {
                task.set_status(Status::Closed);
                Some(task)
            }
        }
    }

    pub fn set_priority(&mut self, id: u32, priority: u32) -> Option<(Task, &Task)> {
        match self.get_mut(id) {
            None => None,
            Some(task) => {
                let old = task.clone();
                task.set_priority(priority);
                Some((old, task))
            }
        }
    }

    pub fn write_all(&self, task_path: &std::path::PathBuf) -> std::io::Result<u32> {
        let mut count = 0;
        for task in self.tasks.iter() {
            Task::write(task, &task_path)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn get_all(&self) -> Vec<&Task> {
        self.tasks.iter().collect()
    }

    pub fn get_open(&self) -> Vec<&Task> {
        self.filter(Status::Open)
    }

    pub fn get_closed(&self) -> Vec<&Task> {
        self.filter(Status::Closed)
    }

    pub fn filter(&self, status: Status) -> Vec<&Task> {
        let iter = self.tasks.iter();
        let filtered_tasks: Vec<&Task> = iter.filter(|t| t.status() == status).collect();
        filtered_tasks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl TaskList {
        pub fn new() -> TaskList {
            TaskList { tasks: vec![] }
        }
    }

    #[test]
    fn add_task() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            tasks = mtasks;
        }

        let t = tasks.get(1).unwrap();
        assert_eq!(1, t.id());
        assert_eq!(1, t.priority());
        assert_eq!("test", t.name());
        assert_eq!(Status::Open, t.status());

        let t2 = tasks.get(2).unwrap();
        assert_eq!(2, t2.id());
        assert_eq!(2, t2.priority());
        assert_eq!("test 2", t2.name());
        assert_eq!(Status::Open, t2.status());
    }

    #[test]
    fn close_task() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let t = tasks.get(1).unwrap();
        assert_eq!(1, t.id());
        assert_eq!(1, t.priority());
        assert_eq!("test", t.name());
        assert_eq!(Status::Closed, t.status());

        let t2 = tasks.get(2).unwrap();
        assert_eq!(2, t2.id());
        assert_eq!(2, t2.priority());
        assert_eq!("test 2", t2.name());
        assert_eq!(Status::Open, t2.status());
    }

    #[test]
    fn change_priority() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            let (old, new) = mtasks.set_priority(t, 4).expect("The old and new tasks");
            assert_eq!(1, old.priority());
            assert_eq!(1, old.id());
            assert_eq!("test", old.name());
            assert_eq!(4, new.priority());
            assert_eq!(1, new.id());
            assert_eq!("test", new.name());

            // set priority for a task which does not exist
            match mtasks.set_priority(3, 2) {
                None => (),
                Some(_) => panic!("None should be returned when changing priority for a task which does not exist"),
            }

            tasks = mtasks;
        }

        let t = tasks.get(1).unwrap();
        assert_eq!(1, t.id());
        assert_eq!(4, t.priority());
        assert_eq!("test", t.name());
        assert_eq!(Status::Open, t.status());

        let t2 = tasks.get(2).unwrap();
        assert_eq!(2, t2.id());
        assert_eq!(2, t2.priority());
        assert_eq!("test 2", t2.name());
        assert_eq!(Status::Open, t2.status());
    }

    #[test]
    fn get_open() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let filtered_tasks = tasks.get_open();
        assert_eq!(1, filtered_tasks.len());
        assert_eq!("test 2", filtered_tasks[0].name());
        assert_eq!(Status::Open, filtered_tasks[0].status());
    }

    #[test]
    fn get_closed() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let filtered_tasks = tasks.get_closed();
        assert_eq!(1, filtered_tasks.len());
        assert_eq!("test", filtered_tasks[0].name());
        assert_eq!(Status::Closed, filtered_tasks[0].status());
    }

    #[test]
    fn get_all() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let filtered_tasks = tasks.get_all();
        assert_eq!(2, filtered_tasks.len());
        assert_eq!("test", filtered_tasks[0].name());
        assert_eq!(Status::Closed, filtered_tasks[0].status());
        assert_eq!("test 2", filtered_tasks[1].name());
        assert_eq!(Status::Open, filtered_tasks[1].status());
    }

    #[test]
    fn task_notes() {
        let mut mtasks = TaskList::new();
        let t = mtasks.add_task("test", 1);
        let task = mtasks.get_mut(t).expect("Task not created");

        task.add_note("Test Note");
        let notes = task.notes();
        assert_eq!(1, notes.len());
        assert_eq!("Test Note", notes[0].note());

        task.add_note("Second Note");
        let notes = task.notes();
        assert_eq!(2, notes.len());
        assert_eq!("Test Note", notes[0].note());
        assert_eq!("Second Note", notes[1].note());
    }
}
