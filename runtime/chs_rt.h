#ifndef CHS_RT_H
#define CHS_RT_H
/* CHS runtime for stage-0 C backend (native stage-1 without clang). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

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
