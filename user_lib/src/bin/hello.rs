#![no_std]
#![no_main]


use macros::main;

#[main]
async fn main() -> i32 {
    println!("into user test");
    println!("into user test");
    println!("into user test");
    async {
        println!("inner async");
    }.await;
    0
}
