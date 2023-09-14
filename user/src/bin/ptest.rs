#![no_std]
#![no_main]


#[macro_use]
extern crate user_lib;
extern crate alloc;



#[no_mangle]
pub fn main() -> i32 {
    println!("Basic test: prio_test");
    vdso::spawn(test6, 6, executor::CoroutineKind::Norm);
    vdso::spawn(test5, 5, executor::CoroutineKind::Norm);
    vdso::spawn(test4, 4, executor::CoroutineKind::Norm);
    vdso::spawn(test3, 3, executor::CoroutineKind::Norm);
    vdso::spawn(test2, 2, executor::CoroutineKind::Norm);
    vdso::spawn(test1, 1, executor::CoroutineKind::Norm);    
    0
}



async fn test1() {
    println!("this coroutine shoule run {}", vdso::current_cid(false));
}

async fn test2() {
    println!("this coroutine shoule run {}", vdso::current_cid(false));
}

async fn test3() {
    println!("this coroutine shoule run {}", vdso::current_cid(false));
}

async fn test4() {
    println!("this coroutine shoule run {}", vdso::current_cid(false));
}

async fn test5() {
    println!("this coroutine shoule run {}", vdso::current_cid(false));
}

async fn test6() {
    println!("this coroutine shoule run {}", vdso::current_cid(false));
}


