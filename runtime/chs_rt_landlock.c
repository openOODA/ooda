#include "chs_rt.h"
#include "chs_rt_landlock.h"
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

#ifndef PR_SET_NO_NEW_PRIVS
#define PR_SET_NO_NEW_PRIVS 38
#endif

#ifndef __NR_landlock_create_ruleset
#if defined(__x86_64__)
#define __NR_landlock_create_ruleset 444
#define __NR_landlock_add_rule 445
#define __NR_landlock_restrict_self 446
#elif defined(__aarch64__)
#define __NR_landlock_create_ruleset 444
#define __NR_landlock_add_rule 445
#define __NR_landlock_restrict_self 446
#endif
#endif

#ifndef LANDLOCK_RULE_PATH_BENEATH
#define LANDLOCK_RULE_PATH_BENEATH 1
struct landlock_path_beneath_attr {
  unsigned long long allowed_access;
  int parent_fd;
} __attribute__((packed));

struct landlock_ruleset_attr {
  unsigned long long handled_access_fs;
  unsigned long long handled_access_net;
} __attribute__((packed));
#endif

#define ACCESS_FS_READ (1ULL | 2ULL | 4ULL | 8ULL)
#define ACCESS_FS_WRITE (16ULL | 32ULL | 64ULL | 128ULL | 256ULL | 512ULL | 1024ULL)

int oo_landlock_is_available(void) {
#if defined(__NR_landlock_create_ruleset)
  int abi = (int)syscall(__NR_landlock_create_ruleset, NULL, 0, 1);
  return abi > 0;
#else
  return 0;
#endif
}

OoResS oo_landlock_restrict(long long cap, OoStr read_dirs, OoStr write_dirs) {
  oo_cap_require_sys(cap, "landlock_restrict");
  (void)read_dirs;
  (void)write_dirs;
  if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) {
    return (OoResS){0, oo_str_lit("ERR\tlandlock\tfailed to set PR_SET_NO_NEW_PRIVS")};
  }
#if defined(__NR_landlock_create_ruleset)
  if (!oo_landlock_is_available()) {
    return (OoResS){1, oo_str_lit("OK_NO_NEW_PRIVS")};
  }
  struct landlock_ruleset_attr attr;
  memset(&attr, 0, sizeof attr);
  attr.handled_access_fs = ACCESS_FS_READ | ACCESS_FS_WRITE;
  int ruleset_fd = (int)syscall(__NR_landlock_create_ruleset, &attr, sizeof attr, 0);
  if (ruleset_fd < 0) {
    return (OoResS){1, oo_str_lit("OK_NO_NEW_PRIVS")};
  }
  if (syscall(__NR_landlock_restrict_self, ruleset_fd, 0) != 0) {
    close(ruleset_fd);
    return (OoResS){0, oo_str_lit("ERR\tlandlock\trestrict_self failed")};
  }
  close(ruleset_fd);
  return (OoResS){1, oo_str_lit("OK_LANDLOCK_ENFORCED")};
#else
  return (OoResS){1, oo_str_lit("OK_NO_NEW_PRIVS")};
#endif
}
