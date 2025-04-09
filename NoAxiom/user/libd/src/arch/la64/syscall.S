    .section  .text
    .globl    __syscall
    .align    4
    __syscall:
    move      $a7, $a0
    move      $a0, $a1
    move      $a1, $a2
    move      $a2, $a3
    move      $a3, $a4
    move      $a4, $a5
    syscall   0
    jr        $ra
