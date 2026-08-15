#include "chs_rt.h"
#include "chs_rt_rlimit.h"
#include <sys/resource.h>
#include <sys/time.h>
#include <errno.h>
#include <string.h>

OoResS oo_rlimit_set_mem_mb(long long cap, long long megabytes) {
  oo_cap_require_sys(cap, "rlimit_set_mem");
  if (megabytes <= 0) {
    return (OoResS){0, oo_str_lit("ERR\trlimit\tinvalid memory quota size")};
  }
  struct rlimit rl;
  rlim_t bytes = (rlim_t)megabytes * 1024ULL * 1024ULL;
  rl.rlim_cur = bytes;
  rl.rlim_max = bytes;
  if (setrlimit(RLIMIT_AS, &rl) != 0) {
    return (OoResS){0, oo_str_lit("ERR\trlimit\tfailed to set RLIMIT_AS")};
  }
  return (OoResS){1, oo_str_lit("OK")};
}

OoResS oo_rlimit_set_nofile(long long cap, long long max_fds) {
  oo_cap_require_sys(cap, "rlimit_set_nofile");
  if (max_fds <= 0) {
    return (OoResS){0, oo_str_lit("ERR\trlimit\tinvalid fd count")};
  }
  struct rlimit rl;
  rl.rlim_cur = (rlim_t)max_fds;
  rl.rlim_max = (rlim_t)max_fds;
  if (setrlimit(RLIMIT_NOFILE, &rl) != 0) {
    return (OoResS){0, oo_str_lit("ERR\trlimit\tfailed to set RLIMIT_NOFILE")};
  }
  return (OoResS){1, oo_str_lit("OK")};
}

OoResS oo_rlimit_set_cpu_sec(long long cap, long long seconds) {
  oo_cap_require_sys(cap, "rlimit_set_cpu");
  if (seconds <= 0) {
    return (OoResS){0, oo_str_lit("ERR\trlimit\tinvalid cpu seconds")};
  }
  struct rlimit rl;
  rl.rlim_cur = (rlim_t)seconds;
  rl.rlim_max = (rlim_t)seconds;
  if (setrlimit(RLIMIT_CPU, &rl) != 0) {
    return (OoResS){0, oo_str_lit("ERR\trlimit\tfailed to set RLIMIT_CPU")};
  }
  return (OoResS){1, oo_str_lit("OK")};
}
