#![no_std]
#![no_main]



#[macro_use]
extern crate user_lib;
extern crate alloc;

static mut NUM: usize = 0;

#[no_mangle]
pub fn main() -> i32 {
    println!("Basic test: prio_test");
    vdso::spawn(test6, 6, executor::CoroutineKind::Norm);
    vdso::spawn(test5, 5, executor::CoroutineKind::Norm);
    vdso::spawn(test4, 4, executor::CoroutineKind::Norm);
    vdso::spawn(test3, 3, executor::CoroutineKind::Norm);
    vdso::spawn(test2, 2, executor::CoroutineKind::Norm);
    vdso::spawn(test1, 1, executor::CoroutineKind::Norm);
    vdso::add_vcpu(2);
    println!("wait, NUM is {}", unsafe { NUM });
    0
}



async fn test1() {
    unsafe{ NUM += 1; }
    println!("wait, NUM is {}", unsafe { NUM });
}

async fn test2() {
    unsafe{ NUM += 1; }
    println!("wait, NUM is {}", unsafe { NUM });
}

async fn test3() {
    unsafe{ NUM += 1; }
    println!("wait, NUM is {}", unsafe { NUM });
}

async fn test4() {
    unsafe{ NUM += 1; }
    println!("wait, NUM is {}", unsafe { NUM });
}

async fn test5() {
    unsafe{ NUM += 1; }
    println!("wait, NUM is {}", unsafe { NUM });
}

async fn test6() {
    unsafe{ NUM += 1; }
    println!("wait, NUM is {}", unsafe { NUM });
}


