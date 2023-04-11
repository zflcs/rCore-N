
.section .data.table
.global rcore_symbol_table
.global rcore_symbol_table_size
rcore_symbol_table:
    .zero 1048576
rcore_symbol_table_size:
    .zero 32