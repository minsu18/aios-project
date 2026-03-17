/* AIOS kernel linker script for Raspberry Pi 3/4 (aarch64)
 * Load address: 0x80000 (where GPU loads kernel8.img)
 */
SECTIONS
{
    . = 0x80000;

    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss (NOLOAD) : {
        . = ALIGN(16);
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
        __bss_end = .;
        __stack_bottom = .;
        . = . + 0x10000;  /* 64KB stack */
        __stack_top = .;
    }

    _end = .;

    /DISCARD/ : {
        *(.comment)
        *(.gnu*)
        *(.note*)
        *(.eh_frame*)
    }
}

__bss_size = (__bss_end - __bss_start) / 8;
