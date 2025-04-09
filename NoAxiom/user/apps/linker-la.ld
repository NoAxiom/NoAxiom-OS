OUTPUT_ARCH("loongarch64")
ENTRY(_start)

SECTIONS
{
    . = 0x120000000;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(0x1000);
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(0x1000);
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}
