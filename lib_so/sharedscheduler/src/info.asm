# 必须按照这个格式来写 string，不能换行或者缩进
# 否则内核解析模块信息时会出错
.section .rcore-lkm
.string "name:sharedscheduler
version:1
api_version:1
exported_symbols:init_module,spawn
dependence:
";