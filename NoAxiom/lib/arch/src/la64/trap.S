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

    .section  .text.trampoline
    .globl    __user_trapvec
    .globl    __user_trapret
    .globl    __kernel_trapvec
    .align    4

# user -> kernel
    __user_trapvec:
    csrwr     $sp, CSR_SAVE            # take the kernel sp

# store user genereal regs
    st.d      $ra, $sp, 1*8            # x1 / ra (skip x0)
    st.d      $tp, $sp, 2*8            # x2 / tp
    .set      n, 4                     # skip x3 / sp, start from x4, sp will be saved outside from macro
    .rept     28
    STORE_X   %n
    .set      n, n+1
    .endr
    csrrd     $t0, CSR_PRMD            # LA_PRMD   <> sstatus
    csrrd     $t1, CSR_ERA             # LA_ERA    <> sepc
    csrrd     $t2, CSR_SAVE            # LA_tmp_sp <> sscratch
    st.d      $t0, $sp, 32*8
    st.d      $t1, $sp, 33*8
    st.d      $t2, $sp, 3*8            # store sp (x3 in la64)

# load kernel regs
    ld.d      $ra, $sp, 35*8           # load kernel_sp
    .set      n, 0                     # load callee-saved regs
    .set      offset, 36
    .rept     10                       # only save s0~s9
    LOAD_S    %n, %offset
    .set      n, n+1
    .set      offset, offset+1
    .endr
    ld.d      $fp, $sp, 48*8           # load kernel fp
    ld.d      $tp, $sp, 49*8           # load kernel tp
    ld.d      $sp, $sp, 34*8           # load kernel sp
    jirl      $zero, $ra, 0            # return to kernel ra

# kernel -> user
    __user_trapret:
    csrwr     $a0, CSR_SAVE
    csrrd     $a0, CSR_SAVE            # to use correct a0

# store kernel regs
    st.d      $sp, $a0, 34*8           # save kernel callee-saved reg
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

# load user general regs
    ld.d      $t0, $sp, 32*8
    ld.d      $t1, $sp, 33*8
    csrwr     $t0, CSR_PRMD            # PRMD
    csrwr     $t1, CSR_ERA             # ERA
    ld.d      $ra, $sp, 1*8            # x1 / ra (skip x0)
    ld.d      $tp, $sp, 2*8            # x2 / tp
    .set      n, 4                     # skip x3 / sp, start from x4
    .rept     28
    LOAD_X    %n
    .set      n, n+1
    .endr
    ld.d      $sp, $sp, 3*8            # load sp finally

    ertn                               # la: exception return <> rv: sret

# kernel -> kernel
# only need to save caller-saved regs
# note that we don't save ERA & ECFG or other csrs here
    __kernel_trapvec:
    addi.d    $sp, $sp, -19*8
    st.d      $ra, $sp, 1*8
    st.d      $t0, $sp, 2*8
    st.d      $t1, $sp, 3*8
    st.d      $t2, $sp, 4*8
    st.d      $t3, $sp, 5*8
    st.d      $t4, $sp, 6*8
    st.d      $t5, $sp, 7*8
    st.d      $t6, $sp, 8*8            # diff with rv: t7 & t8 should be saved as well
    st.d      $a0, $sp, 9*8
    st.d      $a1, $sp, 10*8
    st.d      $a2, $sp, 11*8
    st.d      $a3, $sp, 12*8
    st.d      $a4, $sp, 13*8
    st.d      $a5, $sp, 14*8
    st.d      $a6, $sp, 15*8
    st.d      $a7, $sp, 16*8
    st.d      $t7, $sp, 17*8
    st.d      $t8, $sp, 18*8
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
    ld.d      $t7, $sp, 17*8
    ld.d      $t8, $sp, 18*8
    addi.d    $sp, $sp, 19*8
    ertn

# float registers store & load macros
    FP_START  = 0
    FP_END    = 32
    .macro    SAVE_FP n, m
    fst.d     $f\n, $a0, \m*8
    .endm
    .macro    LOAD_FP n, m
    fld.d     $f\n, $a0, \m*8
    .endm

    .globl    __save_freg
    .globl    __load_freg
    .align    4

# save float registers
    __save_freg:
    .set      n, 0
    .set      m, FP_START
    .rept     32
    SAVE_FP   %n, %m                   # save freg
    .set      n, n+1
    .set      m, m+1
    .endr

    movfcsr2gr $t0, $fcsr0             # save FCSR
    st.w      $t0, $a0, FP_END*8

    movcf2gr  $t0, $fcc7               # save FCC
    slli.w    $t0, $t0, 1
    movcf2gr  $t0, $fcc6
    slli.w    $t0, $t0, 1
    movcf2gr  $t0, $fcc5
    slli.w    $t0, $t0, 1
    movcf2gr  $t0, $fcc4
    slli.w    $t0, $t0, 1
    movcf2gr  $t0, $fcc3
    slli.w    $t0, $t0, 1
    movcf2gr  $t0, $fcc2
    slli.w    $t0, $t0, 1
    movcf2gr  $t0, $fcc1
    slli.w    $t0, $t0, 1
    movcf2gr  $t0, $fcc0
    st.b      $t0, $a0, FP_END*8+4     # offset: 32*freg+1*fcsr

# load float registers
    __load_freg:
    .set      n, 0
    .set      m, FP_START
    .rept     32
    LOAD_FP   %n, %m
    .set      n, n+1
    .set      m, m+1
    .endr

    ld.w      $t0, $a0, FP_END*8       # restore FCSR
    movgr2fcsr $fcsr0, $t0

    ld.b      $t0, $a0, FP_END*8+4     # restore FCC
    movgr2cf  $fcc0, $t0
    srli.w    $t0, $t0, 1
    movgr2cf  $fcc1, $t0
    srli.w    $t0, $t0, 1
    movgr2cf  $fcc2, $t0
    srli.w    $t0, $t0, 1
    movgr2cf  $fcc3, $t0
    srli.w    $t0, $t0, 1
    movgr2cf  $fcc4, $t0
    srli.w    $t0, $t0, 1
    movgr2cf  $fcc5, $t0
    srli.w    $t0, $t0, 1
    movgr2cf  $fcc6, $t0
    srli.w    $t0, $t0, 1
    movgr2cf  $fcc7, $t0

    .section  .text.signal
    .globl    user_sigreturn
    .align    12
    user_sigreturn:
    ori       $a7, $zero, 139          # syscall SIGRETURN
    syscall   0
