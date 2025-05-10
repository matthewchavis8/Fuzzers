/**
 * Memory Map to instruct the baremetal OS 
 * how to layout the kernel image in the chip's RAM or Flash memory
 *
 * - Tells the linker what memory regions exist(RAM, FLASH, ETC)
 * - How big each region is and where it starts
 * - Where each section of compile code or data goes within each region
 * - what special symbols (entry points, start/end markers) to create
 *
 */

MEMORY 
{
  RAM (xrw) : ORIGIN = 0x00000000, LENGTH = 4M
  
  /* *
   *  Disable for now right now everything will be stored on RAM
   *  FLASH (xr) : ORIGIN = 0x00000000, LENGTH = 4M 
   */
}

/* When the CPU resets resume execution at Reset_Handler */
ENTRY(Reset_Handler)

_Min_Heap_Size = 0x300000;           /* Required amount of heap. */
_Min_Stack_Size = 0x4000;            /* Required amount of stack. */
M_VECTOR_RAM_SIZE = (16 + 48) * 4;   /* Required amound of Ram needed to relocate interrupt table */
_estack = ORIGIN(RAM) + LENGTH(RAM); /* Carves out bytes for interrupts RAM */

SECTIONS
{
    /* Place interupt vector table at the start*/
    .isr_vector :
    {
        __vector_table = .;
        KEEP(*(.isr_vector))
        . = ALIGN(4);
    } > RAM /* FLASH */
    
    /* Read only data goes here */
    .text :
    {
        . = ALIGN(4);
        *(.text*)
        KEEP (*(.init))
        KEEP (*(.fini))
        KEEP(*(.eh_frame))
        *(.rodata*)
        . = ALIGN(4);
        _etext = .;
    } > RAM /* FLASH */
    
    /* Unwind stack tables for C++ exceptions or other ARM exceptions */
    .ARM.extab :
    {
        . = ALIGN(4);
        *(.ARM.extab* .gnu.linkonce.armextab.*)
        . = ALIGN(4);
    } >RAM /* FLASH */
     
    /* Unwind stack tables for C++ exceptions or other ARM exceptions */
    .ARM :
    {
        . = ALIGN(4);
        __exidx_start = .;
        *(.ARM.exidx* .gnu.linkonce.armexidx.*)
        __exidx_end = .;
        . = ALIGN(4);
    } >RAM /* FLASH */
    
    /* Reserves a chunk in RAM to copy or locate the interrupt table */
    .interrupts_ram :
    {
        . = ALIGN(4);
        __VECTOR_RAM__ = .;
        __interrupts_ram_start__ = .;
        . += M_VECTOR_RAM_SIZE;
        . = ALIGN(4);
        __interrupts_ram_end = .;
    } > RAM

    _sidata = LOADADDR(.data);
    
    /* Where global/static variables live */
    .data : /* AT ( _sidata ) */
    {
        . = ALIGN(4);
        _sdata = .;
        *(.data*)
        . = ALIGN(4);
        _edata = .;
    } > RAM /* RAM AT > FLASH */
    
    /* Where special buffers will live in*/
    .uninitialized (NOLOAD):
    {
        . = ALIGN(32);
        __uninitialized_start = .;
        *(.uninitialized)
        KEEP(*(.keep.uninitialized))
        . = ALIGN(32);
        __uninitialized_end = .;
    } > RAM
    
    /* Where zero intialized data lives*/
    .bss :
    {
        . = ALIGN(4);
        _sbss = .;
        __bss_start__ = _sbss;
        *(.bss*)
        *(COMMON)
        . = ALIGN(4);
        _ebss = .;
        __bss_end__ = _ebss;
    } >RAM
    
    /* Where heap memory lives */
    .heap :
    {
        . = ALIGN(8);
        PROVIDE ( end = . );
        PROVIDE ( _end = . );
        _heap_bottom = .;
        . = . + _Min_Heap_Size;
        _heap_top = .;
        . = . + _Min_Stack_Size;
        . = ALIGN(8);
       } >RAM

   /* Set stack top to end of RAM, and stack limit move down by
    * size of stack_dummy section */
   __StackTop = ORIGIN(RAM) + LENGTH(RAM);
   __StackLimit = __StackTop - _Min_Stack_Size;
   PROVIDE(__stack = __StackTop);
     
  /* Check if data + heap + stack exceeds RAM limit */
  ASSERT(__StackLimit >= _heap_top, "region RAM overflowed with stack")
}
