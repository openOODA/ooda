#include "chs_rt.h"

/* Path A M165: owned string ops (byte index). Not &str borrow / no lifetime. */
int oo_str_starts_with(OoStr s, OoStr pre) {
  if (pre.len <= 0) return 1;
  if (!s.data || !pre.data || pre.len > s.len) return 0;
  return memcmp(s.data, pre.data, (size_t)pre.len) == 0;
}

int oo_str_ends_with(OoStr s, OoStr suf) {
  if (suf.len <= 0) return 1;
  if (!s.data || !suf.data || suf.len > s.len) return 0;
  return memcmp(s.data + (s.len - suf.len), suf.data, (size_t)suf.len) == 0;
}

long long oo_str_index_of(OoStr s, OoStr sub) {
  if (sub.len <= 0) return 0;
  if (!s.data || !sub.data || sub.len > s.len) return -1;
  for (long long i = 0; i + sub.len <= s.len; i++)
    if (memcmp(s.data + i, sub.data, (size_t)sub.len) == 0) return i;
  return -1;
}

/* Cap n<=1024; refuse len*n past soft 1<<28 payload ceiling. */
OoStr oo_str_repeat(OoStr s, long long n) {
  if (n < 0) n = 0;
  if (n > 1024) n = 1024;
  long long sl = (s.data && s.len > 0 && s.len < (1LL << 28)) ? s.len : 0;
  if (sl > 0 && n > 0 && sl > ((1LL << 28) / n)) n = (1LL << 28) / sl;
  OoStr r;
  r.len = sl * n;
  r.data = oo_str_alloc_payload((size_t)r.len);
  for (long long i = 0; i < n; i++)
    if (sl > 0) memcpy(r.data + (size_t)(i * sl), s.data, (size_t)sl);
  return r;
}
