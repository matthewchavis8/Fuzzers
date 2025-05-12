
/**
 * Bare-metal ARM Cortex-M startup file
 *
 *
 */

typedef unsigned int uint32_t;

extern int main();

extern uint32_t _estack, _sidata, _sdata, _edata, _sbss, _ebss;

/* Prevent optimization so gcc does not replace code with memcpy */
__attribute__((optimize("O0"))) __attribute__((naked)) void Reset_Handler(
    void) {
  /* set stack pointer */
  __asm volatile("ldr r0, =_estack");
  __asm volatile("mov sp, r0");

  /* copy .data section from flash to RAM */
  // Not needed for this example, see linker script
  // for( uint32_t * src = &_sidata, * dest = &_sdata; dest < &_edata; )
  // {
  //     *dest++ = *src++;
  // }

  /* zero out .bss section */
  for (uint32_t *dest = &_sbss; dest < &_ebss;) {
    *dest++ = 0;
  }

  /* jump to board initialisation */
  void _start(void);
  _start();
}

const uint32_t *isr_vector[] __attribute__((section(".isr_vector"))) = {
    (uint32_t *)&_estack,
    (uint32_t *)&Reset_Handler, /* Reset                -15 */
    0,                          /* NMI_Handler          -14 */
    0,                          /* HardFault_Handler    -13 */
    0,                          /* MemManage_Handler    -12 */
    0,                          /* BusFault_Handler     -11 */
    0,                          /* UsageFault_Handler   -10 */
    0,                          /* reserved */
    0,                          /* reserved */
    0,                          /* reserved */
    0,                          /* reserved   -6 */
    0,                          /* SVC_Handler              -5 */
    0,                          /* DebugMon_Handler         -4 */
    0,                          /* reserved */
    0,                          /* PendSV handler    -2 */
    0,                          /* SysTick_Handler   -1 */
    0,                          /* uart0 receive 0 */
    0,                          /* uart0 transmit */
    0,                          /* uart1 receive */
    0,                          /* uart1 transmit */
    0,                          /* uart 2 receive */
    0,                          /* uart 2 transmit */
    0,                          /* GPIO 0 combined interrupt */
    0,                          /* GPIO 2 combined interrupt */
    0,                          /* Timer 0 */
    0,                          /* Timer 1 */
    0,                          /* Dial Timer */
    0,                          /* SPI0 SPI1 */
    0,                          /* uart overflow 1, 2,3 */
    0,                          /* Ethernet   13 */
};

__attribute__((naked)) void exit(__attribute__((unused)) int status) {
  /* Force qemu to exit using ARM Semihosting */
  __asm volatile(
      "mov r1, r0\n"
      "cmp r1, #0\n"
      "bne .notclean\n"
      "ldr r1, =0x20026\n" /* ADP_Stopped_ApplicationExit, a clean exit */
      ".notclean:\n"
      "movs r0, #0x18\n" /* SYS_EXIT */
      "bkpt 0xab\n"
      "end: b end\n");
}

void _start(void) {
  main();
  exit(0);
}
