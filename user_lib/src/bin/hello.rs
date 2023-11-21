#![no_std]
#![no_main]


use alloc::string::String;
use executor::TaskType;


rcoren::get_libfn!(pub fn spawn(fut: Box<dyn Future<Output = i32> + 'static + Send + Sync>, priority: u32, task_type: TaskType) {});

// rcoren::get_libfn!(pub fn test() -> i32 {});


#[rcoren::main]
async fn main() -> i32 {
    spawn(Box::new(test()), 0, TaskType::Other);
    println!("into user test");
    println!("into user test");
    println!("into user test");
    async {
        println!("inner async");
        for _i in 0..10 {
            let string = String::from("sdasdfaf");
            println!("{}", string);
        }
    }.await;
    0
}

async fn test() -> i32 {
    log::debug!("here");
    0
}

