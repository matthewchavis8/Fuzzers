
#include <stdint.h>
#include <stdlib.h>

#define MAP_SIZE 100

uint8_t map[MAP_SIZE];

uint8_t* map_ptr = map;

// intializes our map to 0
int init() {
  for (int i = 0; i < MAP_SIZE; ++i) {
    map[i] = 0;
  }
   return 0;
}

// Marks coverage map
int mark_mp(uint8_t idx) {
  map[idx] = 1;

  return 0;
}

// Our C harness
void c_harness(uint8_t* buff) {
  mark_mp(0);

  if (buff[0] == 'm') {
    mark_mp(1);
    if (buff[1] == 'a') {
      mark_mp(2);
      if (buff[3] == 't') { 
        mark_mp(3);
        if (buff[4] == 't') {
          abort();
        }
      }
    }
  }
}
