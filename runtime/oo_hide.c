/* Load-time hide table from process entropy. Not live .text rewrite. */
#include "oo_hide.h"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#if defined(__linux__) || defined(__APPLE__)
#include <sys/random.h>
#endif

static unsigned long long g_tab[OO_HIDE_SLOTS];
static int g_ready;
static int g_on;
static int g_ctor_ran;

int oo_hide_enabled(void) {
  const char *a = getenv("OODA_HIDE");
  if (a && a[0] == '1' && a[1] == '\0') {
    return 1;
  }
  return 0;
}

static void hide_fill(void) {
  int i;
  int j;
  unsigned char b[8 * OO_HIDE_SLOTS];
  unsigned long long acc;
  unsigned long long v;
  g_on = oo_hide_enabled();
  if (!g_on) {
    memset(g_tab, 0, sizeof g_tab);
    g_ready = 1;
    return;
  }
  memset(b, 0, sizeof b);
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)getpid();
    acc ^= (unsigned long long)(uintptr_t)g_tab;
    for (i = 0; i < (int)sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  for (i = 0; i < OO_HIDE_SLOTS; i++) {
    v = 0;
    for (j = 0; j < 8; j++) {
      v = (v << 8) | (unsigned long long)b[i * 8 + j];
    }
    if (v == 0) {
      v = 1;
    }
    g_tab[i] = v;
  }
  g_ready = 1;
}

#if defined(__GNUC__)
static void oo_hide_ctor(void) __attribute__((constructor));
static void oo_hide_ctor(void) {
  hide_fill();
  g_ctor_ran = 1;
}
#endif

int oo_hide_loaded_at_start(void) {
  return (g_ctor_ran != 0 && g_ready != 0) ? 1 : 0;
}

int oo_hide_ready(void) {
  return g_ready != 0 ? 1 : 0;
}

void oo_hide_table(unsigned long long *out, int n) {
  int i;
  if (out == 0) {
    return;
  }
  if (n <= 0) {
    return;
  }
  /* No first-call fill. Table exists only if the load ctor ran. */
  if (n > OO_HIDE_SLOTS) {
    n = OO_HIDE_SLOTS;
  }
  for (i = 0; i < n; i++) {
    out[i] = g_ready ? g_tab[i] : 0;
  }
}

unsigned long long oo_hide_fingerprint(void) {
  unsigned long long fp;
  int i;
  fp = 0;
  if (!g_ready) {
    return 0;
  }
  for (i = 0; i < OO_HIDE_SLOTS; i++) {
    fp ^= g_tab[i] + (unsigned long long)(i + 1);
  }
  return fp;
}
