# kernel/trap/trap.S

# TrapContext structure:
#   0 - gpr
# 256 - sstatus
# 264 - sepc
# 272 - kernel_satp
# 280 - kernel_ra
# 288 - trap_handler

    .altmacro
    .macro    STORE_X n
    sd        x\n, \n*8(sp)
    .endm

    .macro    LOAD_X n
    ld        x\n, \n*8(sp)
    .endm

    .macro    STORE_S n, offset
    sd        s\n, \offset*8(a0)
    .endm

    .macro    LOAD_S n, offset
    ld        s\n, \offset*8(sp)
    .endm

    .macro    STORE_GENERAL_REG
    sd        ra, 1*8(sp)
# sd gp, 3*8(sp)
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

    .section  .text.trampoline
    .globl    user_trapvec
    .globl    user_trapret
    .globl    trap_from_kernel
    .align    2


# user -> kernel
    user_trapvec:
    csrrw     sp, sscratch, sp

    STORE_GENERAL_REG
    sd        t2, 2*8(sp)

# move to kernel_sp
    ld        ra, 35*8(sp)

# load callee-saved regs
    .set      n, 0
    .set      offset, 36
    .rept     12
    LOAD_S    %n offset
    .set      n, n+1
    .set      offset, offset+1
    .endr

# load kernel fp
    ld        fp, 48*8(sp)
    ld        tp, 49*8(sp)
    ld        sp, 34*8(sp)

# return to kernel ra
    ret


# kernel -> user
    user_trapret:

    csrw      sscratch, a0

# save kernel callee-saved regs
    sd        sp, 34*8(a0)
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

# to use macros
    mv        sp, a0
    LOAD_GENERAL_REG

    sret

# kernel -> kernel
    trap_from_kernel:
# only need to save caller-saved regs
# note that we don't save sepc & stvec here
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
    call      kernel_trap_handler
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
    sret

    .section  .text.signal
    .globl    user_sigreturn
    .align    12
    user_sigreturn:
    li        a7, 139                  # syscall SIGRETURN
    ecall
