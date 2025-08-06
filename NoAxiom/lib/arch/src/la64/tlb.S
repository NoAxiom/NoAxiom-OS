# fixme: `addi.d $t0, $t0, -1` asserts there's only valid bit in flags
# PGD: 0x1b CRMD:0x0 PWCL:0x1c TLBRBADV:0x89 TLBERA:0x8a TLBRSAVE:0x8b SAVE:0x30
# TLBREHi: 0x8e STLBPS: 0x1e MERRsave:0x95
# reference: https://gitlab.eduxiji.net/educg-group-22027-2376549/T202410699992496-1562/-/blob/NPUcore-FF/os/src/arch/la64/trap/mod.rs#L43
# notes: some consts are added to make the code more readable

    .equ      LA_CSR_CRMD,          0x00 # Current mode
    .equ      LA_CSR_TLBEHI,        0x11 # TLB entry high
    .equ      LA_CSR_TLBELO0,       0x12 # TLB entry low 0
    .equ      LA_CSR_TLBELO1,       0x13 # TLB entry low 1
    .equ      LA_CSR_PGDL,          0x19 # Page table base address when VA[47] = 0
    .equ      LA_CSR_PGDH,          0x1a # Page table base address when VA[47] = 1
    .equ      LA_CSR_PGD,           0x1b # Page table base
    .equ      LA_CSR_TLBRENTRY,     0x88 # TLB refill exception entry
    .equ      LA_CSR_TLBRBADV,      0x89 # TLB refill badvaddr
    .equ      LA_CSR_TLBRERA,       0x8a # TLB refill ERA
    .equ      LA_CSR_TLBRSAVE,      0x8b # KScratch for TLB refill exception
    .equ      LA_CSR_TLBRELO0,      0x8c # TLB refill entrylo0
    .equ      LA_CSR_TLBRELO1,      0x8d # TLB refill entrylo1
    .equ      LA_CSR_TLBREHI,       0x8e # TLB refill entryhi

    .globl    __tlb_refill
    .balign   4096

    __tlb_refill:
    csrwr     $t0, LA_CSR_TLBRSAVE
    csrrd     $t0, LA_CSR_PGD
    lddir     $t0, $t0, 3
    beqz      $t0, 1f
    lddir     $t0, $t0, 2
    beqz      $t0, 1f
    lddir     $t0, $t0, 1
    beqz      $t0, 1f

    ldpte     $t0, 0
    ldpte     $t0, 1
    csrrd     $t0, LA_CSR_TLBRELO0
    csrrd     $t0, LA_CSR_TLBRELO1
    csrrd     $t0, LA_CSR_CRMD

    2:
    tlbfill
    csrrd     $t0, LA_CSR_TLBRBADV
    srli.d    $t0, $t0, 13
    slli.d    $t0, $t0, 13
    csrwr     $t0, LA_CSR_TLBEHI
    tlbsrch
    tlbrd
    csrrd     $t0, LA_CSR_TLBELO0
    csrrd     $t0, LA_CSR_TLBELO1
    csrrd     $t0, LA_CSR_TLBRSAVE
    ertn

    1:
    csrrd     $t0, LA_CSR_TLBREHI
    ori       $t0, $t0, 0xC
    csrwr     $t0, LA_CSR_TLBREHI

    rotri.d   $t0, $t0, 61
    ori       $t0, $t0, 3
    rotri.d   $t0, $t0, 3

    csrwr     $t0, LA_CSR_TLBRELO0
    csrrd     $t0, LA_CSR_TLBRELO0
    csrwr     $t0, LA_CSR_TLBRELO1
    b         2b

#     __tlb_refill:
#     csrwr     $t0, LA_CSR_TLBRSAVE     # store temporary data

# # walk the page table
#     csrrd     $t0, LA_CSR_PGD
#     lddir     $t0, $t0, 3              # load: (39, 30]
#     andi      $t0, $t0, 1
#     beqz      $t0, 1f                  # valid check

#     csrrd     $t0, LA_CSR_PGD
#     lddir     $t0, $t0, 3              # reload: (39, 30]
#     addi.d    $t0, $t0, -1             # sub valid flag
#     lddir     $t0, $t0, 1              # load: (30, 21]
#     andi      $t0, $t0, 1
#     beqz      $t0, 1f                  # valid check

#     csrrd     $t0, LA_CSR_PGD          # reload: (39, 30]
#     lddir     $t0, $t0, 3              # reload: (30, 21]
#     addi.d    $t0, $t0, -1
#     lddir     $t0, $t0, 1              # load: (21, 12]
#     addi.d    $t0, $t0, -1             # no need to check validality since we're in the lowest level

#     ldpte     $t0, 0                   # even page pte, will update TLBRELO0
#     ldpte     $t0, 1                   # odd page pte, will update TLBRELO1
# # csrrd     $t0, LA_CSR_TLBRELO0
# # csrrd     $t0, LA_CSR_TLBRELO1
# # csrrd     $t0, 0x0                 # fixme: is these instructions necessary? it just read the csr value

# # complete the TLB refill and return
#     2:
#     tlbfill
#     csrrd     $t0, LA_CSR_TLBRBADV
#     srli.d    $t0, $t0, 13
#     slli.d    $t0, $t0, 13
#     csrwr     $t0, LA_CSR_TLBEHI
#     tlbsrch
#     tlbrd
#     csrrd     $t0, LA_CSR_TLBELO0
#     csrrd     $t0, LA_CSR_TLBELO1
#     csrwr     $t0, LA_CSR_TLBRSAVE     # restore temporary data, use csrwr to avoid data loss
#     ertn

# # will trigger page fault
#     1:
#     csrrd     $t0, LA_CSR_TLBREHI
#     ori       $t0, $t0, 0xC
#     csrwr     $t0, LA_CSR_TLBREHI

#     rotri.d   $t0, $t0, 61
#     ori       $t0, $t0, 3              # fixme: is 3 correct? should be 7?
#     rotri.d   $t0, $t0, 3

#     csrwr     $t0, LA_CSR_TLBRELO0
#     csrrd     $t0, LA_CSR_TLBRELO0
#     csrwr     $t0, LA_CSR_TLBRELO1
#     b         2b
