
### The Director of user applications

```
.
├── macros
└── src
    └── bin
```

#### Macro

This crate provide the `#[main]` macro used in application. It whill help to create the `heap` and `executor` and init them. Then it will create the main async function.


#### How to write a user application

We must use the `main` macro, it let us directly use the `async` keyword in the main async function.

```rust
#![no_std]
#![no_main]

use macros::main;

#[main]
async fn main() -> i32 {
    ......
    async {
        println!("inner async");
    }.await;
    0
}
```
