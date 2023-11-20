#![no_std]
#![no_main]


use alloc::string::String;
use macros::main;

#[main]
async fn main() -> i32 {
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
