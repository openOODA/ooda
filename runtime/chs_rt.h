#ifndef CHS_RT_H
#define CHS_RT_H
/* Runtime ABI v0 — C realization (Backend-C).
 * See bootstrap/FLOOR.md and bootstrap/RUNTIME_ABI_v0.md.
 * Not a Rust host; thin OS floor under pure .oo emit-c + gcc. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <time.h>

long long oo_monotonic_us(void);

typedef struct {
  char *data;
  long long len;
} OoStr;

typedef struct {
  long long *data;
  long long len;
  long long cap;
} OoIList;

typedef struct {
  OoStr *data;
  long long len;
  long long cap;
} OoSList;


typedef struct {
  int ok;
  OoStr val;
} OoResS;

typedef struct {
  int ok;
  OoStr err;
} OoResV;

#endif
