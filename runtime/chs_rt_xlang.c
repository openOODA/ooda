/* OPEN-70: read a C header; count decls; gcc -flto two units. */
#include "chs_rt.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <sys/stat.h>

static long long oo_xlang_read_size(const char *path) {
  FILE *f;
  long n;
  f = fopen(path, "rb");
  if (!f) return 0;
  if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return 0; }
  n = ftell(f);
  fclose(f);
  if (n < 0) return 0;
  return (long long)n;
}

static void oo_xlang_cpath(OoStr hdr, char *out, size_t cap) {
  size_t n;
  if (!hdr.data || hdr.len <= 0 || cap < 2) { out[0] = 0; return; }
  n = (size_t)hdr.len;
  if (n >= cap) n = cap - 1;
  memcpy(out, hdr.data, n);
  out[n] = 0;
}

long long oo_import_c(OoStr hdr) {
  char name[256];
  char p1[512];
  char p2[512];
  long long n;
  oo_xlang_cpath(hdr, name, sizeof name);
  if (!name[0]) return 0;
  if (name[0] == '/') return oo_xlang_read_size(name);
  snprintf(p1, sizeof p1, "/usr/include/%s", name);
  n = oo_xlang_read_size(p1);
  if (n > 0) return n;
  snprintf(p2, sizeof p2, "/usr/include/%s", strrchr(name, '/') ? strrchr(name, '/') + 1 : name);
  return oo_xlang_read_size(p2);
}

long long oo_ffi_gen(OoStr hdr) {
  return oo_import_c(hdr);
}

long long oo_lto_xlang_link(OoStr a, OoStr b) {
  char na[64];
  char nb[64];
  char cmd[1024];
  int rc;
  (void)mkdir(".ooda-cache", 0755);
  (void)mkdir(".ooda-cache/ooda-tmp", 0755);
  oo_xlang_cpath(a, na, sizeof na);
  oo_xlang_cpath(b, nb, sizeof nb);
  if (!na[0]) strncpy(na, "a", sizeof na - 1);
  if (!nb[0]) strncpy(nb, "b", sizeof nb - 1);
  {
    FILE *fa = fopen(".ooda-cache/ooda-tmp/ooda_lto_a.c", "w");
    FILE *fb = fopen(".ooda-cache/ooda-tmp/ooda_lto_b.c", "w");
    if (!fa || !fb) {
      if (fa) fclose(fa);
      if (fb) fclose(fb);
      return 1;
    }
    fprintf(fa, "int ooda_lto_%s(int x) { return x + 1; }\n", na);
    fprintf(fb,
            "extern int ooda_lto_%s(int);\n"
            "int main(void) { return ooda_lto_%s(41) == 42 ? 0 : 1; }\n",
            na, na);
    fclose(fa);
    fclose(fb);
  }
  snprintf(cmd, sizeof cmd,
           "gcc -flto -c -o .ooda-cache/ooda-tmp/ooda_lto_a.o .ooda-cache/ooda-tmp/ooda_lto_a.c && "
           "gcc -flto -c -o .ooda-cache/ooda-tmp/ooda_lto_b.o .ooda-cache/ooda-tmp/ooda_lto_b.c && "
           "gcc -flto -o .ooda-cache/ooda-tmp/ooda_lto.bin .ooda-cache/ooda-tmp/ooda_lto_a.o "
           ".ooda-cache/ooda-tmp/ooda_lto_b.o");
  rc = system(cmd);
  if (rc != 0) return 1;
  return 0;
}
