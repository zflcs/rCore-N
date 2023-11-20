### rCore-N

This project is based from [duskmoon314/rCore-N](https://github.com/duskmoon314/rCore-N/tree/master)

Add support of **Async Runtime**.


#### Workspace

```
├── easy-fs                 // The file system used in kernel
├── easy-fs-fuse            // The package function which can be used in `std`
├── executor                // The structures related to async runtime
├── lang                    // The basic functions which are required by rust
├── lib                     // The director of dynamic shared objects
│   └── sharedscheduler     // The async runtime, which is a shared object
├── opensbi                 // kernel will run on this sbi
├── os                      // kernel
├── script                  // scripts in test
├── syscall
│   ├── syscall_macro       // the macro to generate the related syscall functions
└── user_lib
    ├── macros              // the `main` macro used in user apps
    └── src
        └── bin             // user apps
```

More infomations can be founded in the readme in the respective directory. 


#### Build the environment

##### QEMU with user trap support

The [QEMU](https://github.com/U-interrupt/qemu) has been added N Extension CSRS of RISC-V support.


```bash
git clone https://github.com/U-interrupt/qemu
mkdir qemu-build
cd qemu-build
../qemu/configure --target-list="riscv64-softmmu"
make -j8
```

##### RISC-V Toolchain

You must install the `riscv64-unknown-linux-gnu-` toolchain. We support a compiled toolchain in [toolchain](https://share.weiyun.com/4CIkApk1) 


#### Build and run

```bash
make run
```

