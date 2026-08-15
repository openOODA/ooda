/* CHS runtime umbrella — single TU for existing gcc command lines */
#include "chs_rt_str.c"
#include "chs_rt_str_intern.c"
#include "chs_rt_str_int.c"
#include "chs_rt_str_tok.c"
#include "chs_rt_str_ops.c"
#include "chs_rt_emit_tmp.c"
#include "chs_rt_list.c"
#include "chs_rt_caps.c"
#include "chs_rt_sys.c"
#include "chs_rt_fetch.c"
#include "chs_rt_ffi_sec.c"
#include "chs_rt_ffi.c"
#include "chs_rt_hash.c"
#include "chs_rt_netfloor.c"
#include "chs_rt_tls.c"
#include "chs_rt_libfloor.c"
#include "chs_rt_thread.c"
#include "chs_rt_channel.c"
#include "chs_rt_actor.c"
#include "chs_rt_hitl.c"
#include "chs_rt_time_rand.c"
#include "chs_rt_alloc.c"
#include "chs_rt_arena.c"
#include "chs_rt_fs.c"
#include "chs_rt_print.c"
#include "chs_rt_math.c"
#include "chs_rt_crypto.c"
#include "chs_rt_meta.c"
#include "chs_rt_host.c"
#include "chs_rt_rlimit.c"
#include "chs_rt_wasm.c"
#include "chs_rt_landlock.c"
#include "chs_rt_dns.c"

OoSList str_split(OoStr s, OoStr delim) {
  OoSList l = oo_slist_new();
  if (!s.data || s.len <= 0) return l;
  if (!delim.data || delim.len <= 0) {
    OoSList next = oo_slist_push(l, s);
    oo_slist_release(l);
    return next;
  }
  long long start = 0;
  for (long long i = 0; i + delim.len <= s.len; i++) {
    if (memcmp(s.data + i, delim.data, (size_t)delim.len) == 0) {
      OoStr part;
      part.len = i - start;
      part.data = oo_str_alloc_payload((size_t)part.len);
      if (part.len > 0) memcpy(part.data, s.data + start, (size_t)part.len);
      OoSList next = oo_slist_push(l, part);
      oo_slist_release(l);
      l = next;
      oo_str_release(part);
      i += delim.len - 1;
      start = i + 1;
    }
  }
  OoStr part;
  part.len = s.len - start;
  part.data = oo_str_alloc_payload((size_t)part.len);
  if (part.len > 0) memcpy(part.data, s.data + start, (size_t)part.len);
  OoSList next = oo_slist_push(l, part);
  oo_slist_release(l);
  l = next;
  oo_str_release(part);
  return l;
}

OoStr str_trim(OoStr s) {
  if (!s.data || s.len <= 0) {
    OoStr r;
    r.len = 0;
    r.data = oo_str_alloc_payload(0);
    return r;
  }
  long long start = 0;
  while (start < s.len && isspace((unsigned char)s.data[start])) {
    start++;
  }
  long long end = s.len;
  while (end > start && isspace((unsigned char)s.data[end - 1])) {
    end--;
  }
  OoStr r;
  r.len = end - start;
  r.data = oo_str_alloc_payload((size_t)r.len);
  if (r.len > 0) memcpy(r.data, s.data + start, (size_t)r.len);
  return r;
}
