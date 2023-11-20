
### The Director of user applications

```
.
├── src
└── syscall_macro
    └── src
```

#### Macro

- The `GenSysMacro` and `GenSysTrait` macro help us to generate the user syscall interface and the kernel syscall trait.
- The `#[async_fn]` macro will help us to generate the asynchronous and synchronous syscall function.


#### How to add a syscall

We directly add a enum in the  `SyscallId` structure, the macro will help us add a new syscall. The arguments of the syscall are specificed in the `#[arguments]` attribute.

```rust
pub enum SyscallId {
    #[arguments(args = "fd")]
    Dup = 24,
    #[arguments(args = "path_ptr, flag_bits")]
    Open = 56,
    #[arguments(args = "fd")]
    Close = 57,
    #[arguments(args = "pipe_ptr")]
    Pipe = 59,
    #[arguments(args = "fd, buffer_ptr, buffer_len, key, cid")]
    Read = 63,
    #[arguments(args = "fd, buffer_ptr, buffer_len, key, cid")]
    Write = 64,
    #[arguments(args = "exit_code")]
    Exit = 93,
}
```
