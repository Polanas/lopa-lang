#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
  int32_t x;
  int32_t y;
} Point;

int main() {
  int32_t *ints = malloc(sizeof(int32_t) * 2);
  ints[0] = 2;
  ints[1] = 3;
  printf("%i %i\n", ints[0], ints[1]);
  return 0;
}
