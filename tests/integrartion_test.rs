/**
 * Note about test execution: tests must be run with `--test-threads=1` so that
 * only one test is run at a time.  These tests all involve creation of files and
 * manipulation of directories and the current directory of the process; running
 * tests in parallel will cause them to interfere with each other and break in
 * unexpected ways
 */

extern crate tisk;

use std::fs;
use std::env;
use std::path::PathBuf;

fn setup(test: &str) -> String {
    let root = format!("/tmp/test/{}", test);
    fs::create_dir_all(format!("{}/a", root)).unwrap();
    fs::create_dir_all(format!("{}/a/b", root)).unwrap();
    fs::create_dir_all(format!("{}/a/c", root)).unwrap();
    root
}

fn teardown(test: &str) {
    let root = format!("/tmp/test/{}", test);
    println!("Removing: {}", root);
    match fs::remove_dir_all(root) {
        Ok(_) => (),
        Err(_) => (),
    }
}

#[test]
fn test_upsearch() {
    let root_dir = setup("test_upsearch");
    let proj_dir = format!("{}/a", root_dir);
    let task_dir = format!("{}/.task", proj_dir);
    fs::create_dir_all(&task_dir).unwrap();

    let expected_path = PathBuf::from(task_dir).canonicalize().unwrap();

    // test that upsearch finds .task if it is in the current directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&proj_dir).unwrap();
    let path = tisk::up_search(".", ".task").unwrap().unwrap();
    assert_eq!(expected_path, path);

    // test that upsearch finds .task if it is the parent directory
    env::set_current_dir(&original_dir).unwrap();
    env::set_current_dir(&proj_dir).unwrap();
    let path = tisk::up_search(".", ".task").unwrap().unwrap();
    assert_eq!(expected_path, path);

    // test that upsearch finds .task if it is more than one ancestor away
    env::set_current_dir(&original_dir).unwrap();
    env::set_current_dir(&proj_dir).unwrap();
    let path = tisk::up_search(".", ".task").unwrap().unwrap();
    assert_eq!(expected_path, path);

    // test that upsearch returns None if there is no .task in any parent directory
    env::set_current_dir(&original_dir).unwrap();
    env::set_current_dir(&root_dir).unwrap();
    let path = tisk::up_search(".", ".task").unwrap();
    assert_eq!(None, path);

    env::set_current_dir(&original_dir).unwrap();

    teardown("test_upsearch");
}

#[test]
fn test_initialize() {
    let root_dir = setup("test_init");
    let proj_dir = format!("{}/a", root_dir);
    let expected_task_dir = format!("{}/.task", proj_dir);
    let original_dir = env::current_dir().unwrap();

    env::set_current_dir(&proj_dir).unwrap();

    let result = tisk::initialize().unwrap();
    assert_eq!(tisk::InitResult::Initialized, result);
    env::set_current_dir(&expected_task_dir).unwrap();  // test that the directory was actually created
    
    env::set_current_dir(&original_dir).unwrap();
    let result = tisk::initialize().unwrap();
    assert_eq!(tisk::InitResult::AlreadyInitialized, result);
    env::set_current_dir(&expected_task_dir).unwrap();  // test that the directory was actually created

    env::set_current_dir(&original_dir).unwrap();

    teardown("test_init");
}

#[test]
fn test_add_task() {
    let original_dir = env::current_dir().unwrap();
    teardown("test_add_task");
    let root_dir = setup("test_add_task");
    let proj_dir = format!("{}/a", root_dir);

    env::set_current_dir(&proj_dir).unwrap();

    let result = tisk::initialize().unwrap();
    assert_eq!(tisk::InitResult::Initialized, result);

    let task_path = tisk::up_search(".", ".task").unwrap().unwrap();
    {
        // read a task project 
        // and add a new task and write the new task
        let mut tasks = tisk::TaskList::read_tasks(&task_path).unwrap();
        tasks.add_task("a test task").expect("failed to add task");
        tasks.write_all(&task_path).unwrap();
    }

    {
        // Load the task project again 
        // and validate that the expected task is there
        // then close the task
        let mut tasks = tisk::TaskList::read_tasks(&task_path).unwrap();
        let task = tasks.get(1).unwrap();
        assert_eq!("a test task", task.name());
        assert_eq!(tisk::Status::Open, task.status());
        tasks.close_task(1).unwrap();
        tasks.write_all(&task_path).unwrap();
    }

    {
        // Load the task project again 
        // and validate that the task was closed
        // then add a new task
        let mut tasks = tisk::TaskList::read_tasks(&task_path).unwrap();
        let task = tasks.get(1).unwrap();
        assert_eq!("a test task", task.name());
        assert_eq!(tisk::Status::Closed, task.status());
        tasks.add_task("a second test task").expect("failed to add task");
        tasks.write_all(&task_path).unwrap();
    }

    {
        // Load the task project again 
        // and validate that the two tasks are there
        let tasks = tisk::TaskList::read_tasks(&task_path).unwrap();
        let task = tasks.get(1).unwrap();
        assert_eq!("a test task", task.name());
        assert_eq!(tisk::Status::Closed, task.status());

        let task = tasks.get(2).unwrap();
        assert_eq!("a second test task", task.name());
        assert_eq!(tisk::Status::Open, task.status());
    }

    env::set_current_dir(&original_dir).unwrap();
    teardown("test_add_task");
}
