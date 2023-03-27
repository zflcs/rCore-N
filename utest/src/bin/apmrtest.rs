#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

// not in SUCC_TESTS & FAIL_TESTS
// count_lines, infloop, user_shell, usertests

// item of TESTS : app_name(argv_0), argv_1, argv_2, argv_3, exit_code
static TOTAL_TEST: &[(&str, &str, &str, &str, i32)] = &[
    ("apmr\0", "1\0", "1\0", "\0", 0),        // 线程数 1、数据量 1
    ("apmr\0", "2\0", "1\0", "\0", 0),        // 线程数 2、数据量 1
    ("apmr\0", "3\0", "1\0", "\0", 0),        // 线程数 3、数据量 1
    ("apmr\0", "4\0", "1\0", "\0", 0),        // 线程数 4、数据量 1
    ("apmr\0", "5\0", "1\0", "\0", 0),        // 线程数 5、数据量 1
    ("apmr\0", "1\0", "228\0", "\0", 0),        // 线程数 1、数据量 228
    ("apmr\0", "2\0", "228\0", "\0", 0),        // 线程数 2、数据量 228
    ("apmr\0", "3\0", "228\0", "\0", 0),        // 线程数 2、数据量 228
    ("apmr\0", "4\0", "228\0", "\0", 0),        // 线程数 2、数据量 228
    ("apmr\0", "5\0", "228\0", "\0", 0),        // 线程数 2、数据量 228
    ("apmr\0", "1\0", "912\0", "\0", 0),        // 线程数 1、数据量 912
    ("apmr\0", "2\0", "912\0", "\0", 0),        // 线程数 2、数据量 912
    ("apmr\0", "3\0", "912\0", "\0", 0),        // 线程数 2、数据量 912
    ("apmr\0", "4\0", "912\0", "\0", 0),        // 线程数 2、数据量 912
    ("apmr\0", "5\0", "912\0", "\0", 0),        // 线程数 2、数据量 912
    ("apmr\0", "1\0", "3876\0", "\0", 0),        // 线程数 1、数据量 3876
    ("apmr\0", "2\0", "3876\0", "\0", 0),        // 线程数 2、数据量 3876
    ("apmr\0", "3\0", "3876\0", "\0", 0),        // 线程数 2、数据量 3876
    ("apmr\0", "4\0", "3876\0", "\0", 0),        // 线程数 2、数据量 3876
    ("apmr\0", "5\0", "3876\0", "\0", 0),        // 线程数 2、数据量 3876
];

static RUN_CYCLE: usize = 10; 

use user_lib::{exec, fork, waitpid};

fn run_tests(tests: &[(&str, &str, &str, &str, i32)]) -> i32 {
    let mut pass_num = 0;
    let mut arr: [*const u8; 4] = [
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
    ];

    for test in tests {
        println!("\x1b[33mUsertests: Running {}\x1b[0m", test.0);
        for i in 0..RUN_CYCLE{
            println!("\x1b[34mRun {}---{}\x1b[0m", test.0, i);
            arr[0] = test.0.as_ptr();
            if test.1 != "\0" {
                arr[1] = test.1.as_ptr();
                arr[2] = core::ptr::null::<u8>();
                arr[3] = core::ptr::null::<u8>();
                if test.2 != "\0" {
                    arr[2] = test.2.as_ptr();
                    arr[3] = core::ptr::null::<u8>();
                    if test.3 != "\0" {
                        arr[3] = test.3.as_ptr();
                    } else {
                        arr[3] = core::ptr::null::<u8>();
                    }
                } else {
                    arr[2] = core::ptr::null::<u8>();
                    arr[3] = core::ptr::null::<u8>();
                }
            } else {
                arr[1] = core::ptr::null::<u8>();
                arr[2] = core::ptr::null::<u8>();
                arr[3] = core::ptr::null::<u8>();
            }

            let pid = fork();
            if pid == 0 {
                exec(test.0, &arr[..]);
                panic!("unreachable!");
            } else {
                let mut exit_code: i32 = Default::default();
                let wait_pid = waitpid(pid as usize, &mut exit_code);
                assert_eq!(pid, wait_pid);
                println!(
                    "\x1b[32mUtest: Apmr {} in Process {}, thread {}, data_len {} exited with code {}\x1b[0m",
                    test.0, pid, test.1, test.2, exit_code
                );
            }
        }
    }
    pass_num
}

#[no_mangle]
pub fn main() -> i32 {
    run_tests(TOTAL_TEST);
    0
}