# TrapContext structure:
#   0 - user_reg
# 256 - sstatus
# 264 - sepc
# 272 - kernel_satp
# 280 - kernel_ra
# 288 - kernel_reg (callee_saved)
# 384 - kernel_tp


    .altmacro
    .macro    STORE_X n                # store user regs
    sd        x\n, \n*8(sp)
    .endm

    .macro    LOAD_X n                 # load user regs
    ld        x\n, \n*8(sp)
    .endm

    .macro    STORE_S n, offset        # store kernel callee-saved regs
    sd        s\n, \offset*8(a0)
    .endm

    .macro    LOAD_S n, offset         # load kernel callee-saved regs
    ld        s\n, \offset*8(sp)
    .endm

    .macro    STORE_GENERAL_REG
    sd        ra, 1*8(sp)
    .set      n, 3
    .rept     29
    STORE_X   %n
    .set      n, n+1
    .endr
    csrr      t0, sstatus
    csrr      t1, sepc
    csrr      t2, sscratch
    sd        t0, 32*8(sp)
    sd        t1, 33*8(sp)
    .endm

    .macro    LOAD_GENERAL_REG
    ld        t0, 32*8(sp)
    ld        t1, 33*8(sp)
    csrw      sstatus, t0
    csrw      sepc, t1
    ld        ra, 1*8(sp)
    .set      n, 3
    .rept     29
    LOAD_X    %n
    .set      n, n+1
    .endr
    ld        sp, 2*8(sp)
    .endm

    .macro    KERNEL_SAVE_REG
    addi      sp, sp, -17*8
    sd        ra,  1*8(sp)
    sd        t0,  2*8(sp)
    sd        t1,  3*8(sp)
    sd        t2,  4*8(sp)
    sd        t3,  5*8(sp)
    sd        t4,  6*8(sp)
    sd        t5,  7*8(sp)
    sd        t6,  8*8(sp)
    sd        a0,  9*8(sp)
    sd        a1, 10*8(sp)
    sd        a2, 11*8(sp)
    sd        a3, 12*8(sp)
    sd        a4, 13*8(sp)
    sd        a5, 14*8(sp)
    sd        a6, 15*8(sp)
    sd        a7, 16*8(sp)
    .endm

    .macro    KERNEL_LOAD_REG
    ld        ra,  1*8(sp)
    ld        t0,  2*8(sp)
    ld        t1,  3*8(sp)
    ld        t2,  4*8(sp)
    ld        t3,  5*8(sp)
    ld        t4,  6*8(sp)
    ld        t5,  7*8(sp)
    ld        t6,  8*8(sp)
    ld        a0,  9*8(sp)
    ld        a1, 10*8(sp)
    ld        a2, 11*8(sp)
    ld        a3, 12*8(sp)
    ld        a4, 13*8(sp)
    ld        a5, 14*8(sp)
    ld        a6, 15*8(sp)
    ld        a7, 16*8(sp)
    addi      sp, sp, 17*8
    .endm

    .section  .text.trampoline
    .globl    __user_trapvec
    .globl    __user_trapret
    .globl    __kernel_trapvec
    .globl    __kernel_user_ptr_vec
    .align    2

# user -> kernel
    __user_trapvec:
    csrrw     sp, sscratch, sp
    STORE_GENERAL_REG
    sd        t2, 2*8(sp)
    ld        ra, 35*8(sp)             # move to kernel_sp
    .set      n, 0                     # load callee-saved regs
    .set      offset, 36
    .rept     12
    LOAD_S    %n offset
    .set      n, n+1
    .set      offset, offset+1
    .endr
    ld        fp, 48*8(sp)             # load kernel fp
    ld        tp, 49*8(sp)
    ld        sp, 34*8(sp)
    ret                                # return to kernel ra

# kernel -> user
    __user_trapret:
    csrw      sscratch, a0
    sd        sp, 34*8(a0)             # save kernel callee-saved regs
    sd        ra, 35*8(a0)
    .set      n, 0
    .set      offset, 36
    .rept     12
    STORE_S   %n offset
    .set      n, n+1
    .set      offset, offset+1
    .endr
    sd        fp, 48*8(a0)
    sd        tp, 49*8(a0)
    mv        sp, a0
    LOAD_GENERAL_REG
    sret

# kernel -> kernel
# only need to save caller-saved regs
# note that we don't save sepc & stvec here
    __kernel_trapvec:
    KERNEL_SAVE_REG
    call      rv_kernel_trap_handler
    KERNEL_LOAD_REG
    sret

    .globl    __kernel_user_ptr_vec
    .align    12
    __kernel_user_ptr_vec:
    csrr      a0, sepc
    addi      a0, a0, 4
    csrw      sepc, a0
    li        a0, 1
    csrr      a1, scause
    sret

    .globl    __try_read_user
    .align    4
    __try_read_user:
    mv        a1, a0
    mv        a0, zero
    .option   push
    .option   norvc
    lb        a1, 0(a1)
    .option   pop
    ret

    .globl    __try_write_user
    .align    4
    __try_write_user:
    mv        a2, a0
    mv        a0, zero
    .option   push
    .option   norvc
    lb        a1, 0(a2)
    sb        a1, 0(a2)
    .option   pop
    ret

    .section  .text.signal
    .globl    user_sigreturn
    .align    12
    user_sigreturn:
    li        a7, 139                  # syscall SIGRETURN
    ecall
