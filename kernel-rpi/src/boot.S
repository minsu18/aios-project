/* AIOS RPi boot assembly — entry at 0x80000 */
.section ".text.boot"
.global _start

_start:
    /* Only core 0 runs; others halt */
    mrs x1, mpidr_el1
    and x1, x1, #0xFF
    cbz x1, 2f
1:  wfe
    b 1b

2:  /* Core 0: set stack below _start */
    ldr x1, =_start
    mov sp, x1

    /* Zero BSS */
    ldr x1, =__bss_start
    ldr x2, =__bss_size
3:  cbz x2, 4f
    str xzr, [x1], #8
    sub x2, x2, #1
    b 3b

4:  /* Jump to Rust kernel_main */
    bl kernel_main
    b 1b
