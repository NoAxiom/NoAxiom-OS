    .equ      CSR_SAVE, 0x30
    .equ      CSR_ERA, 0x6
    .equ      CSR_PRMD, 0x1
    .equ      CSR_PGDL, 0x19
    .equ      CSR_PGD, 0x1b

    .altmacro
    .macro    STORE_X n                # store user regs
    st.d      $r\n, $sp, \n*8
    .endm

    .macro    LOAD_X n                 # load user regs
    ld.d      $r\n, $sp, \n*8
    .endm

    .macro    STORE_S n, offset        # store kernel callee-saved regs
    st.d      $s\n, $a0, \offset*8
    .endm

    .macro    LOAD_S n, offset         # load kernel callee-saved regs
    ld.d      $s\n, $sp, \offset*8
    .endm

    .macro    STORE_GENERAL_REG
    st.d      $ra, $sp, 1*8
    .set      n, 3
    .rept     29
    STORE_X   %n
    .set      n, n+1
    .endr
    csrrd     $t0, CSR_PRMD            # LoongArch的PRMD寄存器(类似sstatus)
    csrrd     $t1, CSR_ERA             # LoongArch的ERA寄存器(类似sepc)
    csrrd     $t2, CSR_SAVE            # LoongArch的用户栈指针(类似sscratch)
    st.d      $t0, $sp, 32*8
    st.d      $t1, $sp, 33*8
    .endm

    .macro    LOAD_GENERAL_REG
    ld.d      $t0, $sp, 32*8
    ld.d      $t1, $sp, 33*8
    csrwr     $t0, CSR_PRMD            # 写PRMD
    csrwr     $t1, CSR_ERA             # 写ERA
    ld.d      $ra, $sp, 1*8
    .set      n, 3
    .rept     29
    LOAD_X    %n
    .set      n, n+1
    .endr
    ld.d      $sp, $sp, 2*8
    .endm

    .section  .text.trampoline
    .globl    user_trapvec
    .globl    user_trapret
    .globl    kernel_trapvec
    .align    2

# user -> kernel
    user_trapvec:
    csrwr     $sp, CSR_SAVE            # 交换用户栈指针和内核栈指针
    STORE_GENERAL_REG
    st.d      $t2, $sp, 2*8
    ld.d      $ra, $sp, 35*8           # 移动到kernel_sp
    .set      n, 0                     # 加载callee-saved寄存器
    .set      offset, 36
    .rept     10
    LOAD_S    %n, %offset
    .set      n, n+1
    .set      offset, offset+1
    .endr
    ld.d      $fp, $sp, 48*8           # 加载kernel fp
    ld.d      $tp, $sp, 49*8
    ld.d      $sp, $sp, 34*8
    jirl      $zero, $ra, 0            # 返回到kernel ra

# kernel -> user
    user_trapret:
    csrwr     $a0, CSR_SAVE
    st.d      $sp, $a0, 34*8           # 保存kernel callee-saved寄存器
    st.d      $ra, $a0, 35*8
    .set      n, 0
    .set      offset, 36
    .rept     10
    STORE_S   %n, %offset
    .set      n, n+1
    .set      offset, offset+1
    .endr
    st.d      $fp, $a0, 48*8
    st.d      $tp, $a0, 49*8
    or        $sp, $a0, $zero
    LOAD_GENERAL_REG
    ertn                               # LoongArch的异常返回指令(类似sret)

# kernel -> kernel
# 只需要保存caller-saved寄存器
# 注意这里不保存ERA和ECFG(类似sepc & stvec)
    kernel_trapvec:
    addi.d    $sp, $sp, -17*8
    st.d      $ra, $sp, 1*8
    st.d      $t0, $sp, 2*8
    st.d      $t1, $sp, 3*8
    st.d      $t2, $sp, 4*8
    st.d      $t3, $sp, 5*8
    st.d      $t4, $sp, 6*8
    st.d      $t5, $sp, 7*8
    st.d      $t6, $sp, 8*8
    st.d      $a0, $sp, 9*8
    st.d      $a1, $sp, 10*8
    st.d      $a2, $sp, 11*8
    st.d      $a3, $sp, 12*8
    st.d      $a4, $sp, 13*8
    st.d      $a5, $sp, 14*8
    st.d      $a6, $sp, 15*8
    st.d      $a7, $sp, 16*8
    bl        kernel_trap_handler
    ld.d      $ra, $sp, 1*8
    ld.d      $t0, $sp, 2*8
    ld.d      $t1, $sp, 3*8
    ld.d      $t2, $sp, 4*8
    ld.d      $t3, $sp, 5*8
    ld.d      $t4, $sp, 6*8
    ld.d      $t5, $sp, 7*8
    ld.d      $t6, $sp, 8*8
    ld.d      $a0, $sp, 9*8
    ld.d      $a1, $sp, 10*8
    ld.d      $a2, $sp, 11*8
    ld.d      $a3, $sp, 12*8
    ld.d      $a4, $sp, 13*8
    ld.d      $a5, $sp, 14*8
    ld.d      $a6, $sp, 15*8
    ld.d      $a7, $sp, 16*8
    addi.d    $sp, $sp, 17*8
    ertn

    .section  .text.signal
    .globl    user_sigreturn
    .align    12
    user_sigreturn:
    ori       $a7, $zero, 139          # syscall SIGRETURN
    syscall   0
