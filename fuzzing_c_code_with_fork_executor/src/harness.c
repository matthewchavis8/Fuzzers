#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ipc.h>
#include <sys/shm.h>
#include <sys/types.h>

#define SHMEM_COUNT 100  // Size of our SharedMemoryMap 100 bytes


// Id for our shared memory segment the kernel gives it to you
int shmid;  
// a identifier to thje type of shared-memory segment you want
key_t key = 58974; 


/*
 *  Creates a shared memory segment
 *
 *  @return - returns 0 if it was successful in create a shared 
 *            memory map else -1 if otherwise
 *
 */
int create_shmem_array() {
  // creating our sharedmemory segment and getting the kernel ID to it
  shmid = shmget(key, SHMEM_COUNT * sizeof(uint8_t), IPC_CREAT | 0666);

  // Attaching to our shared memory segment
  void* res = shmat(shmid, NULL, 0);
  // creating our ptr to our shared memory segment to control identifier
  uint8_t* array_ptr = (uint8_t*)res;

  // intializing each byte to 0 in our shared memory map
  for (int i = 0; i < SHMEM_COUNT; ++i) 
    array_ptr[i] = 0;

  return 0;
}

/**
 *  Sets our byte in our shared memory map to 1
 *  marks our coverage
 *
 *
 *  @return - returns 0 if it was successful in setting byte to 1
 */ 

int set_shmem_map(uint8_t idx) {
  
  // Attaching to our shared memory segment
  void* res = shmat(shmid, NULL, 0);
  
  // creating our ptr to our shared memory segment to control identifier
  uint8_t* array_ptr = (uint8_t*)res;
  
  // mark coverage
  array_ptr[idx] = 1;

  return 0;
}

/*
 * gets our value of our coverage map index 
 * checks if it 1 or 0 if it has been seen or not
 *
 * @param idx - index to our memory map
 *
 * @return - returns the value of associated with the index eitheir 1 or 0
 */
int get_shmem_map(uint8_t idx) {
  // Attaching to shared memory segment
  void* res = shmat(shmid, NULL, 0);
  
  // Get the memory segment as a ptr to control
  uint8_t* array_ptr = (uint8_t*)res;

  return array_ptr[idx];
}

/*
 *  Destorys our coverage map
 *
 *  @return - returns 0 if destorying our shared memory map was successful
 */

int destroy_shmem(int id) {
  if (shmctl(id, IPC_RMID, NULL) == -1) 
    return -1;

  return 0;
}

/*
 * harness to our target what we want to fuzz.
 * Panicks on purpose if all coverage has been hit
 */

void c_harness(uint8_t* arr) {
  set_shmem_map(0);

  if (arr[0] == 'a') {
    set_shmem_map(1);
    if (arr[0] == 'b') {
      set_shmem_map(2);
      if (arr[0] == 'c') {
        set_shmem_map(3);
        abort();
      }
    }
  }
}

uint8_t* get_ptr() {
  void* res = shmat(shmid, NULL, 0);

  return (uint8_t*)res;
}
