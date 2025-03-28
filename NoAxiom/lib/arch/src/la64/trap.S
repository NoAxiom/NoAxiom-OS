    .section  .text.trampoline
    .globl    user_trapvec
    .globl    user_trapret
    .globl    kernel_trapvec
    .align    2

    user_trapvec:

    user_trapret:

    kernel_trapvec:

    .section  .text.signal
    .globl    user_sigreturn
    .align    12
    user_sigreturn:
