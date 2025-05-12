/**
 *  These two macros define when we want our fuzzer to start 
 *
 *  Sync-exit mode we tell our fuzzer explicitly when we want to start and stop
 *  Breakpoint mode no explictity instead it enter a infinite loop so the CPU detects and kills the 
 *  Harness
 *
 */
typedef unsigned int uint32_t;

#ifdef TARGET_SYNC_EXIT
  #include "libafl_qemu.h"
#endif /* if def TARGET_SYNC_EXIT */

#ifndef TARGET_SYNC_EXIT
int __attribute__((noinline)) BREAKPOINT() {
  for (;;) {}
}
#endif /* if not def TARGET_SYNC_EXIT */

/**  Standard libFuzzer style entry for one test input
 *   simulates how you would fuzz a baremetal target
 *   example 3 ways fuzzing can go
 *     - Success
 *     - Crashes
 *     - Timeouts
 */
int LLVMFuzzerTestOneInput(uint32_t* data, uint32_t size) {
  #ifdef TARGET_SYNC_EXIT
  // Tells QEMU fuzzer to start collecting coverage from address Data with length Size
   libafl_qemu_start_phys((void*)data, size);
  #endif /* ifdef TARGET_SYNC_EXIT */

  // Timeout Trigger
  if (data[3] == 0)
    while (1) {}

  // Swap each 4 bytes
  for (int i = 0; i < size; ++i) {
    for (int j = i + 1; j < size; ++j) {
      if (data[j] == 0)
        continue;
      if (data[j] > data[i]) {
        int tmp = data[i];
        data[i] = data[j];
        data[j] = tmp;
        if (data[i] <= 100)
          j--;
      }
    }
  }
  #ifdef TARGET_SYNC_EXIT
    // Stops QEMU fuzzer coverage
    libafl_qemu_end(LIBAFL_QEMU_END_OK);
  #else 
    // force a timeout
    return BREAKPOINT();
  #endif /* ifdef TARGET_SYNC_EXIT */
  
  return 1;
}

// uint32_t FUZZ_INPUT[] = {
//   101, 201, 700, 230, 860, 234, 980, 200, 340, 678, 230, 134, 900,
//   236, 900, 123, 800, 123, 658, 607, 246, 804, 567, 568, 207, 407,
//   246, 678, 457, 892, 834, 456, 878, 246, 699, 854, 234, 844, 290,
//   125, 324, 560, 852, 928, 910, 790, 853, 345, 234, 586,
// };

// int main() {
//   LLVMFuzzerTestOneInput(FUZZ_INPUT, 50);
// }
