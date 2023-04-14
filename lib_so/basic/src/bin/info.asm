# 必须按照这个格式来写 string，不能换行或者缩进
# 否则内核解析模块信息时会出错
.section .rcore-lkm
.string "name:sharedscheduler
version:1
api_version:1
exported_symbols:init_module,spawn,user_entry,poll_kernel_future,current_cid,re_back,add_virtual_core,max_prio,reprio,update_prio
dependence:
";