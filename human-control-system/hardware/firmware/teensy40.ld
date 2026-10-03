/*
 * Linker script for Teensy 4.0 (IMXRT1062)
 * 1984KB Flash, 1024KB RAM (512KB DTCM + 512KB OCRAM)
 */

MEMORY
{
    FLASH (rx)  : ORIGIN = 0x60000000, LENGTH = 1984K
    DTCM (rwx)  : ORIGIN = 0x20000000, LENGTH = 512K
    OCRAM (rwx) : ORIGIN = 0x20200000, LENGTH = 512K
}

SECTIONS
{
    .text :
    {
        KEEP(*(.vectors))
        *(.text*)
        *(.rodata*)
        *(.glue_7)
        *(.glue_7t)
        *(.eh_frame)
        . = ALIGN(4);
    } > FLASH

    .data : AT(ADDR(.text) + SIZEOF(.text))
    {
        _sdata = .;
        *(.data*)
        . = ALIGN(4);
        _edata = .;
    } > DTCM

    .bss :
    {
        _sbss = .;
        *(.bss*)
        *(COMMON)
        . = ALIGN(4);
        _ebss = .;
    } > DTCM

    .heap :
    {
        _sheap = .;
        . = . + 16K;
        _eheap = .;
    } > OCRAM

    .stack :
    {
        _sstack = .;
        . = . + 32K;
        _estack = .;
    } > OCRAM

    _estack = ORIGIN(OCRAM) + LENGTH(OCRAM);

    _sidata = LOADADDR(.data);
}