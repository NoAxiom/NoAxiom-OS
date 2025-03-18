OUTPUT_ARCH("loongarch64")
ENTRY(_entry)
BASE_ADDRESS = 0xffffffc080200000;

SECTIONS
{
    . =  BASE_ADDRESS; 
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        . = ALIGN(4K);
        /*
        ssignal = .;
        *(.text.signal);
        . = ALIGN(4K);
        esignal = .;
        */
        *(.text .text.*)
    }

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        . = ALIGN(4K);
        *(.data.prepage .data.prepage.*)
        . = ALIGN(4K);
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;

    sbss = .;
    .bss : {
        skstack = .;
        *(.bss.kstack)
        . = ALIGN(4K);
        ekstack = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}