#include "chs_rt.h"
#include <unistd.h>

/* OPEN-8: in-process oodac --json-errors. Armed by a cwd flag file so the
 * first rebuild can use this runtime without a new .oo builtin. */
static int oo_je_checked = 0;
static int oo_je_on = 0;
static int oo_je_n = 0;
static int oo_je_atexit = 0;
static char oo_je_path[512];

static int oo_je_armed(void) {
  if (!oo_je_checked) {
    FILE *f;
    size_t nrd;
    oo_je_checked = 1;
    oo_je_path[0] = 0;
    f = fopen(".ooda-cache/ooda-tmp/json_errors.arm", "rb");
    if (!f) {
      oo_je_on = 0;
      return 0;
    }
    oo_je_on = 1;
    nrd = fread(oo_je_path, 1, sizeof(oo_je_path) - 1, f);
    fclose(f);
    while (nrd > 0 && (oo_je_path[nrd - 1] == '\n' || oo_je_path[nrd - 1] == '\0')) {
      nrd--;
    }
    oo_je_path[nrd] = 0;
  }
  return oo_je_on;
}

static void oo_je_esc(FILE *f, const char *p, long long n) {
  long long i;
  for (i = 0; i < n; i++) {
    char c = p[i];
    if (c == '\\' || c == '"') {
      fputc('\\', f);
      fputc(c, f);
    } else if (c == '\n') {
      fputs("\\n", f);
    } else if (c == '\t') {
      fputs("\\t", f);
    } else {
      fputc(c, f);
    }
  }
}

static const char *oo_je_code(const char *p, long long n) {
  long long i;
  for (i = 0; i + 10 < n; i++) {
    if (memcmp(p + i, "capability", 10) == 0) return "E_CAP";
  }
  for (i = 0; i + 4 < n; i++) {
    if (memcmp(p + i, "type", 4) == 0 && (i == 0 || p[i - 1] == '\t')) return "E_TC";
  }
  for (i = 0; i + 5 < n; i++) {
    if (memcmp(p + i, "parse", 5) == 0) return "E_PARSE";
  }
  for (i = 0; i + 6 < n; i++) {
    if (memcmp(p + i, "secret", 6) == 0) return "E_SECRET";
  }
  return "E_OTHER";
}

static void oo_je_flush(void) {
  if (!oo_je_on) return;
  if (oo_je_n == 0) {
    fputs("[]\n", stdout);
  } else {
    fputs("]\n", stdout);
  }
  oo_je_on = 0;
  unlink(".ooda-cache/ooda-tmp/json_errors.arm");
}

static void oo_je_loc(const char *p, long long n, long long *line, long long *col) {
  const char *a;
  const char *b;
  long long i;
  *line = 0;
  *col = 0;
  if (!p || n <= 0) return;
  a = NULL;
  for (i = 0; i + 14 <= n; i++) {
    if (memcmp(p + i, "Type error at ", 14) == 0) {
      a = p + i + 14;
      break;
    }
  }
  if (a) {
    *line = 0;
    while (a < p + n && *a >= '0' && *a <= '9') {
      *line = *line * 10 + (*a - '0');
      a++;
    }
    if (a < p + n && *a == ':') {
      a++;
      *col = 0;
      while (a < p + n && *a >= '0' && *a <= '9') {
        *col = *col * 10 + (*a - '0');
        a++;
      }
    }
    return;
  }
  for (i = 0; i + 9 <= n; i++) {
    if (memcmp(p + i, " at line ", 9) == 0) {
      a = p + i + 9;
      *line = 0;
      while (a < p + n && *a >= '0' && *a <= '9') {
        *line = *line * 10 + (*a - '0');
        a++;
      }
      b = a;
      while (b + 6 <= p + n && memcmp(b, ", col ", 6) != 0) b++;
      if (b + 6 <= p + n) {
        b += 6;
        *col = 0;
        while (b < p + n && *b >= '0' && *b <= '9') {
          *col = *col * 10 + (*b - '0');
          b++;
        }
      }
      return;
    }
  }
}

static void oo_je_emit(OoStr s) {
  const char *code;
  long long line = 0;
  long long col = 0;
  if (oo_je_n == 0) fputc('[', stdout);
  else fputc(',', stdout);
  code = oo_je_code(s.data, s.len);
  oo_je_loc(s.data, s.len, &line, &col);
  fputs("{\"code\":\"", stdout);
  fputs(code, stdout);
  fprintf(stdout, "\",\"line\":%lld,\"col\":%lld,\"msg\":\"", line, col);
  oo_je_esc(stdout, s.data, s.len);
  fputs("\",\"path\":\"", stdout);
  oo_je_esc(stdout, oo_je_path, (long long)strlen(oo_je_path));
  fputs("\",\"fix_hint\":\"See openOODA/SHIPPED.oot and ROADMAP.oot.\"", stdout);
  if (strcmp(code, "E_CAP") == 0) {
    fputs(",\"kind\":\"CapabilitySecurityViolation\",\"suggested_fix\":\"Add a matching &Cap parameter\"", stdout);
  }
  fputc('}', stdout);
  oo_je_n++;
}

void oo_print_str(OoStr s) {
  int armed = oo_je_armed();
  if (s.data && s.len >= 4 && armed && memcmp(s.data, "ERR\t", 4) == 0) {
    if (!oo_je_atexit) {
      atexit(oo_je_flush);
      oo_je_atexit = 1;
    }
    oo_je_emit(s);
    return;
  }
  if (s.data && s.len >= 2 && armed && s.data[0] == 'O' && s.data[1] == 'K') {
    if (!oo_je_atexit) {
      atexit(oo_je_flush);
      oo_je_atexit = 1;
    }
    return;
  }
  fwrite(s.data, 1, (size_t)s.len, stdout);
}
void oo_eprint_str(OoStr s) { fwrite(s.data, 1, (size_t)s.len, stderr); }
void oo_print_int(long long n) { printf("%lld", n); }
void oo_print_bool(int b) { fputs(b ? "true" : "false", stdout); }
void oo_println(void) {
  if (oo_je_armed()) {
    if (!oo_je_atexit) {
      atexit(oo_je_flush);
      oo_je_atexit = 1;
    }
    return;
  }
  fputc('\n', stdout);
}
void oo_eprintln(void) { fputc('\n', stderr); }

int oo_str_eq(OoStr a, OoStr b) {
  if (a.len != b.len) return 0;
  return memcmp(a.data, b.data, (size_t)a.len) == 0;
}

int oo_str_contains(OoStr hay, OoStr needle) {
  if (needle.len == 0) return 1;
  if (needle.len > hay.len) return 0;
  /* Length-bounded search so zero-copy slices (no interior NUL) stay correct. */
  for (long long i = 0; i + needle.len <= hay.len; i++) {
    if (memcmp(hay.data + i, needle.data, (size_t)needle.len) == 0) return 1;
  }
  return 0;
}
OoStr oo_int_to_str(long long n) {
  return oo_int_intern(n);
}

OoStr oo_str_trim(OoStr s) {
  if (!s.data || s.len == 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  long long start = 0;
  while (start < s.len && isspace((unsigned char)s.data[start])) start++;
  long long end = s.len;
  while (end > start && isspace((unsigned char)s.data[end - 1])) end--;
  long long tlen = end - start;
  if (tlen <= 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  OoStr r;
  r.len = tlen;
  r.data = oo_str_alloc_payload((size_t)tlen);
  memcpy(r.data, s.data + start, (size_t)tlen);
  return r;
}

OoStr oo_str_to_lowercase(OoStr s) {
  if (!s.data || s.len == 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  int needs = 0;
  for (long long i = 0; i < s.len; i++) {
    unsigned char c = (unsigned char)s.data[i];
    if (c >= 'A' && c <= 'Z') {
      needs = 1;
      break;
    }
  }
  if (!needs) {
    oo_str_retain(s);
    return s;
  }
  OoStr r;
  r.len = s.len;
  r.data = oo_str_alloc_payload((size_t)r.len);
  for (long long i = 0; i < s.len; i++) {
    r.data[i] = (char)tolower((unsigned char)s.data[i]);
  }
  return r;
}

/* Path A M165 free name: owned ASCII upper; not &str. Method: to_lowercase exists. */
OoStr oo_str_to_uppercase(OoStr s) {
  if (!s.data || s.len == 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  int needs = 0;
  for (long long i = 0; i < s.len; i++) {
    unsigned char c = (unsigned char)s.data[i];
    if (c >= 'a' && c <= 'z') {
      needs = 1;
      break;
    }
  }
  if (!needs) {
    oo_str_retain(s);
    return s;
  }
  OoStr r;
  r.len = s.len;
  r.data = oo_str_alloc_payload((size_t)r.len);
  for (long long i = 0; i < s.len; i++) {
    r.data[i] = (char)toupper((unsigned char)s.data[i]);
  }
  return r;
}
