.align 3
.section .module_info
.global smodule_info
.global emodule_info
smodule_info:
    .incbin "/home/zfl/u-intr/rCore-N/lib/sharedscheduler/info.txt"
emodule_info:
