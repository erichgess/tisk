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
    fs::remove_dir_all(root).unwrap();
}


// all the integration tests which create files are kept in a single test function,
// this is to force all the steps to be serialized: otherwise the directory changes
// will interfere with each other when tests are run in parallel
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

    teardown("test_upsearch");
    
    let root_dir = setup("test_init");
    let proj_dir = format!("{}/a", root_dir);
    let expected_task_dir = format!("{}/.task", proj_dir);
    //let expected_task_path = PathBuf::from(expected_task_dir).canonicalize().unwrap();

    env::set_current_dir(&proj_dir).unwrap();

    let result = tisk::initialize().unwrap();
    assert_eq!(tisk::InitResult::Initialized, result);
    env::set_current_dir(&expected_task_dir).unwrap();  // test that the directory was actually created
    
    env::set_current_dir(&original_dir).unwrap();
    let result = tisk::initialize().unwrap();
    assert_eq!(tisk::InitResult::AlreadyInitialized, result);
    env::set_current_dir(&expected_task_dir).unwrap();  // test that the directory was actually created

    teardown("test_init");
}
