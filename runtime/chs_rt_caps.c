#include "chs_rt.h"
#include <errno.h>
#include <sys/types.h>
#include <unistd.h>
#include <pthread.h>
#if defined(__linux__)
#include <sys/random.h>
#endif

/* Process-local capability tokens (R1). Fixed magic ints no longer grant.
 * Time/Rand tokens live in chs_rt_time_rand.c (M12). */
static pthread_once_t g_caps_once = PTHREAD_ONCE_INIT;
static long long g_tok_fs, g_tok_sys, g_tok_env, g_tok_net;
static long long g_tok_sign, g_tok_process, g_tok_sync, g_tok_mem, g_tok_http, g_tok_tcp, g_tok_udp, g_tok_bind, g_tok_audio, g_tok_camera, g_tok_usb, g_tok_hid, g_tok_window, g_tok_frame, g_tok_fsread, g_tok_fswrite;

static void caps_once_init(void) {
  unsigned char b[160];
  size_t i;
  unsigned long long acc;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    /* Fallback: ASLR + pid + clock — still not a fixed published magic. */
    acc = (unsigned long long)(uintptr_t)&g_tok_fs;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  g_tok_fs = 0x1000000000000000LL | (long long)((((unsigned long long)b[0]) << 56) | (((unsigned long long)b[1]) << 48) | (((unsigned long long)b[2]) << 40) | (((unsigned long long)b[3]) << 32) | (((unsigned long long)b[4]) << 24) | (((unsigned long long)b[5]) << 16) | (((unsigned long long)b[6]) << 8) | ((unsigned long long)b[7]));
  g_tok_sys = 0x2000000000000000LL | (long long)((((unsigned long long)b[8]) << 56) | (((unsigned long long)b[9]) << 48) | (((unsigned long long)b[10]) << 40) | (((unsigned long long)b[11]) << 32) | (((unsigned long long)b[12]) << 24) | (((unsigned long long)b[13]) << 16) | (((unsigned long long)b[14]) << 8) | ((unsigned long long)b[15]));
  g_tok_env = 0x3000000000000000LL | (long long)((((unsigned long long)b[16]) << 56) | (((unsigned long long)b[17]) << 48) | (((unsigned long long)b[18]) << 40) | (((unsigned long long)b[19]) << 32) | (((unsigned long long)b[20]) << 24) | (((unsigned long long)b[21]) << 16) | (((unsigned long long)b[22]) << 8) | ((unsigned long long)b[23]));
  g_tok_net = 0x4000000000000000LL | (long long)((((unsigned long long)b[24]) << 56) | (((unsigned long long)b[25]) << 48) | (((unsigned long long)b[26]) << 40) | (((unsigned long long)b[27]) << 32) | (((unsigned long long)b[28]) << 24) | (((unsigned long long)b[29]) << 16) | (((unsigned long long)b[30]) << 8) | ((unsigned long long)b[31]));
  g_tok_sign = ((long long)5ULL << 56) | (long long)((((unsigned long long)b[32]) << 56) | (((unsigned long long)b[33]) << 48) | (((unsigned long long)b[34]) << 40) | (((unsigned long long)b[35]) << 32) | (((unsigned long long)b[36]) << 24) | (((unsigned long long)b[37]) << 16) | (((unsigned long long)b[38]) << 8) | ((unsigned long long)b[39]));
  g_tok_process = ((long long)6ULL << 56) | (long long)((((unsigned long long)b[40]) << 56) | (((unsigned long long)b[41]) << 48) | (((unsigned long long)b[42]) << 40) | (((unsigned long long)b[43]) << 32) | (((unsigned long long)b[44]) << 24) | (((unsigned long long)b[45]) << 16) | (((unsigned long long)b[46]) << 8) | ((unsigned long long)b[47]));
  g_tok_sync = ((long long)7ULL << 56) | (long long)((((unsigned long long)b[48]) << 56) | (((unsigned long long)b[49]) << 48) | (((unsigned long long)b[50]) << 40) | (((unsigned long long)b[51]) << 32) | (((unsigned long long)b[52]) << 24) | (((unsigned long long)b[53]) << 16) | (((unsigned long long)b[54]) << 8) | ((unsigned long long)b[55]));
  g_tok_mem = ((long long)8ULL << 56) | (long long)((((unsigned long long)b[56]) << 56) | (((unsigned long long)b[57]) << 48) | (((unsigned long long)b[58]) << 40) | (((unsigned long long)b[59]) << 32) | (((unsigned long long)b[60]) << 24) | (((unsigned long long)b[61]) << 16) | (((unsigned long long)b[62]) << 8) | ((unsigned long long)b[63]));
  g_tok_http = ((long long)9ULL << 56) | (long long)((((unsigned long long)b[64]) << 56) | (((unsigned long long)b[65]) << 48) | (((unsigned long long)b[66]) << 40) | (((unsigned long long)b[67]) << 32) | (((unsigned long long)b[68]) << 24) | (((unsigned long long)b[69]) << 16) | (((unsigned long long)b[70]) << 8) | ((unsigned long long)b[71]));
  g_tok_tcp = ((long long)10ULL << 56) | (long long)((((unsigned long long)b[72]) << 56) | (((unsigned long long)b[73]) << 48) | (((unsigned long long)b[74]) << 40) | (((unsigned long long)b[75]) << 32) | (((unsigned long long)b[76]) << 24) | (((unsigned long long)b[77]) << 16) | (((unsigned long long)b[78]) << 8) | ((unsigned long long)b[79]));
  g_tok_udp = ((long long)11ULL << 56) | (long long)((((unsigned long long)b[80]) << 56) | (((unsigned long long)b[81]) << 48) | (((unsigned long long)b[82]) << 40) | (((unsigned long long)b[83]) << 32) | (((unsigned long long)b[84]) << 24) | (((unsigned long long)b[85]) << 16) | (((unsigned long long)b[86]) << 8) | ((unsigned long long)b[87]));
  g_tok_bind = ((long long)12ULL << 56) | (long long)((((unsigned long long)b[88]) << 56) | (((unsigned long long)b[89]) << 48) | (((unsigned long long)b[90]) << 40) | (((unsigned long long)b[91]) << 32) | (((unsigned long long)b[92]) << 24) | (((unsigned long long)b[93]) << 16) | (((unsigned long long)b[94]) << 8) | ((unsigned long long)b[95]));
  g_tok_audio = ((long long)13ULL << 56) | (long long)((((unsigned long long)b[96]) << 56) | (((unsigned long long)b[97]) << 48) | (((unsigned long long)b[98]) << 40) | (((unsigned long long)b[99]) << 32) | (((unsigned long long)b[100]) << 24) | (((unsigned long long)b[101]) << 16) | (((unsigned long long)b[102]) << 8) | ((unsigned long long)b[103]));
  g_tok_camera = ((long long)14ULL << 56) | (long long)((((unsigned long long)b[104]) << 56) | (((unsigned long long)b[105]) << 48) | (((unsigned long long)b[106]) << 40) | (((unsigned long long)b[107]) << 32) | (((unsigned long long)b[108]) << 24) | (((unsigned long long)b[109]) << 16) | (((unsigned long long)b[110]) << 8) | ((unsigned long long)b[111]));
  g_tok_usb = ((long long)15ULL << 56) | (long long)((((unsigned long long)b[112]) << 56) | (((unsigned long long)b[113]) << 48) | (((unsigned long long)b[114]) << 40) | (((unsigned long long)b[115]) << 32) | (((unsigned long long)b[116]) << 24) | (((unsigned long long)b[117]) << 16) | (((unsigned long long)b[118]) << 8) | ((unsigned long long)b[119]));
  g_tok_hid = ((long long)16ULL << 56) | (long long)((((unsigned long long)b[120]) << 56) | (((unsigned long long)b[121]) << 48) | (((unsigned long long)b[122]) << 40) | (((unsigned long long)b[123]) << 32) | (((unsigned long long)b[124]) << 24) | (((unsigned long long)b[125]) << 16) | (((unsigned long long)b[126]) << 8) | ((unsigned long long)b[127]));
  g_tok_window = ((long long)17ULL << 56) | (long long)((((unsigned long long)b[128]) << 56) | (((unsigned long long)b[129]) << 48) | (((unsigned long long)b[130]) << 40) | (((unsigned long long)b[131]) << 32) | (((unsigned long long)b[132]) << 24) | (((unsigned long long)b[133]) << 16) | (((unsigned long long)b[134]) << 8) | ((unsigned long long)b[135]));
  g_tok_frame = ((long long)18ULL << 56) | (long long)((((unsigned long long)b[136]) << 56) | (((unsigned long long)b[137]) << 48) | (((unsigned long long)b[138]) << 40) | (((unsigned long long)b[139]) << 32) | (((unsigned long long)b[140]) << 24) | (((unsigned long long)b[141]) << 16) | (((unsigned long long)b[142]) << 8) | ((unsigned long long)b[143]));
  g_tok_fsread = ((long long)19ULL << 56) | (long long)((((unsigned long long)b[144]) << 56) | (((unsigned long long)b[145]) << 48) | (((unsigned long long)b[146]) << 40) | (((unsigned long long)b[147]) << 32) | (((unsigned long long)b[148]) << 24) | (((unsigned long long)b[149]) << 16) | (((unsigned long long)b[150]) << 8) | ((unsigned long long)b[151]));
  g_tok_fswrite = ((long long)20ULL << 56) | (long long)((((unsigned long long)b[152]) << 56) | (((unsigned long long)b[153]) << 48) | (((unsigned long long)b[154]) << 40) | (((unsigned long long)b[155]) << 32) | (((unsigned long long)b[156]) << 24) | (((unsigned long long)b[157]) << 16) | (((unsigned long long)b[158]) << 8) | ((unsigned long long)b[159]));
  /* Never equal classic forgeable magics */
  if (g_tok_fs == 0x4F4F4653LL) g_tok_fs ^= 0x11111111LL;
  if (g_tok_sys == 0x4F4F5359LL) g_tok_sys ^= 0x11111111LL;
  if (g_tok_env == 0x4F4F454ELL) g_tok_env ^= 0x11111111LL;
  if (g_tok_net == 0x4F4F4E54LL) g_tok_net ^= 0x11111111LL;
}

static void oo_caps_init(void) {
  pthread_once(&g_caps_once, caps_once_init);
}

long long oo_cap_grant_fs(void) { oo_caps_init(); return g_tok_fs; }
long long oo_cap_grant_sys(void) { oo_caps_init(); return g_tok_sys; }
long long oo_cap_grant_env(void) { oo_caps_init(); return g_tok_env; }
long long oo_cap_grant_net(void) { oo_caps_init(); return g_tok_net; }
long long oo_cap_grant_sign(void) { oo_caps_init(); return g_tok_sign; }
long long oo_cap_grant_process(void) { oo_caps_init(); return g_tok_process; }
long long oo_cap_grant_sync(void) { oo_caps_init(); return g_tok_sync; }
long long oo_cap_grant_mem(void) { oo_caps_init(); return g_tok_mem; }
long long oo_cap_grant_http(void) { oo_caps_init(); return g_tok_http; }
long long oo_cap_grant_tcp(void) { oo_caps_init(); return g_tok_tcp; }
long long oo_cap_grant_udp(void) { oo_caps_init(); return g_tok_udp; }
long long oo_cap_grant_bind(void) { oo_caps_init(); return g_tok_bind; }
long long oo_cap_grant_audio(void) { oo_caps_init(); return g_tok_audio; }
long long oo_cap_grant_camera(void) { oo_caps_init(); return g_tok_camera; }
long long oo_cap_grant_usb(void) { oo_caps_init(); return g_tok_usb; }
long long oo_cap_grant_hid(void) { oo_caps_init(); return g_tok_hid; }
long long oo_cap_grant_window(void) { oo_caps_init(); return g_tok_window; }
long long oo_cap_grant_frame(void) { oo_caps_init(); return g_tok_frame; }
long long oo_cap_grant_fsread(void) { oo_caps_init(); return g_tok_fsread; }
long long oo_cap_grant_fswrite(void) { oo_caps_init(); return g_tok_fswrite; }

void oo_cap_require(long long got, long long want, const char *op) {
  oo_caps_init();
  if (got != want) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "?");
    exit(1);
  }
}

void oo_cap_require_fs(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_fs) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "fs");
    exit(1);
  }
}
void oo_cap_require_sys(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_sys) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "sys");
    exit(1);
  }
}
void oo_cap_require_env(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_env) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "env");
    exit(1);
  }
}
void oo_cap_require_net(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_net) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "net");
    exit(1);
  }
}

void oo_cap_require_http(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_http && got != g_tok_net) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "http");
    exit(1);
  }
}
void oo_cap_require_tcp(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_tcp && got != g_tok_net) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "tcp");
    exit(1);
  }
}
void oo_cap_require_udp(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_udp && got != g_tok_net) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "udp");
    exit(1);
  }
}
void oo_cap_require_bind(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_bind && got != g_tok_net) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "bind");
    exit(1);
  }
}
void oo_cap_require_fsread(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_fsread && got != g_tok_fs) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "fsread");
    exit(1);
  }
}
void oo_cap_require_fswrite(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_fswrite && got != g_tok_fs) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "fswrite");
    exit(1);
  }
}
void oo_cap_require_sign(long long got, const char *op) {
  oo_cap_require(got, g_tok_sign, op ? op : "sign");
}
void oo_cap_require_process(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_process && got != g_tok_sys) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n",
            op ? op : "process");
    exit(1);
  }
}
void oo_cap_require_sync(long long got, const char *op) {
  oo_cap_require(got, g_tok_sync, op ? op : "sync");
}
void oo_cap_require_mem(long long got, const char *op) {
  oo_cap_require(got, g_tok_mem, op ? op : "mem");
}
void oo_cap_require_audio(long long got, const char *op) {
  oo_cap_require(got, g_tok_audio, op ? op : "audio");
}
void oo_cap_require_camera(long long got, const char *op) {
  oo_cap_require(got, g_tok_camera, op ? op : "camera");
}
void oo_cap_require_usb(long long got, const char *op) {
  oo_cap_require(got, g_tok_usb, op ? op : "usb");
}
void oo_cap_require_hid(long long got, const char *op) {
  oo_cap_require(got, g_tok_hid, op ? op : "hid");
}
void oo_cap_require_window(long long got, const char *op) {
  oo_cap_require(got, g_tok_window, op ? op : "window");
}
void oo_cap_require_frame(long long got, const char *op) {
  oo_cap_require(got, g_tok_frame, op ? op : "frame");
}
