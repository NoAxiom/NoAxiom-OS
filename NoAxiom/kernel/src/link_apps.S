
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad 3
    .quad app_0_start
    .quad app_1_start
    .quad app_2_start
    .quad app_2_end

    .section .data
    .global app_0_start
    .global app_0_end
app_0_start:
    .incbin "./NoAxiom/user/apps/hello_world_0.elf"
app_0_end:

    .section .data
    .global app_1_start
    .global app_1_end
app_1_start:
    .incbin "./NoAxiom/user/apps/hello_world_1.elf"
app_1_end:

    .section .data
    .global app_2_start
    .global app_2_end
app_2_start:
    .incbin "./NoAxiom/user/apps/hello_world_2.elf"
app_2_end:
