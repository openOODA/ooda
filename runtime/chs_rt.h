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
#include <stdint.h>

long long oo_monotonic_us(void);

#define OO_FLAG_STATIC 1

typedef struct {
  uint32_t ref_count;
  uint32_t flags;
} OoStrHeader;

typedef struct {
  uint32_t ref_count;
  uint32_t flags;
} OoListHeader;

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

/* Internal payload allocation helpers.
 * Ambient List quota: default 64MiB; override OO_LIST_AMBIENT_QUOTA (bytes).
 * Not OS rlimit; raise ceiling via alloc_bytes(&AllocCap, n).
 * Per-raise n capped at 1<<30 in chs_rt_alloc.c (oversize → no-op); residual:
 * still not setrlimit / cgroup / ASAN heap isolation. */
extern long long oo_list_ambient_quota;
extern long long oo_list_ambient_bytes;
void oo_list_quota_init_public(void);
char *oo_str_alloc_payload(size_t len);
void *oo_list_alloc_payload(size_t elem_size, size_t cap);

/* Retain/release APIs */
void oo_str_retain(OoStr s);
void oo_str_release(OoStr s);
void oo_ilist_retain(OoIList l);
void oo_ilist_release(OoIList l);
void oo_slist_retain(OoSList l);
void oo_slist_release(OoSList l);

/* String API */
OoStr oo_str_lit(const char *s);
OoStr oo_str_concat(OoStr a, OoStr b);
long long oo_str_byte_len(OoStr s);
long long oo_chars_len(OoStr s);
OoStr oo_char_at(OoStr s, long long idx);
OoStr oo_str_slice(OoStr s, long long start, long long end);
int oo_char_is_digit(OoStr s);
int oo_char_is_alpha(OoStr s);
int oo_char_is_space(OoStr s);
OoStr oo_int_to_str(long long n);
OoStr oo_str_trim(OoStr s);
OoStr oo_str_to_lowercase(OoStr s);
OoStr oo_str_to_uppercase(OoStr s);
int oo_str_eq(OoStr a, OoStr b);
int oo_str_contains(OoStr hay, OoStr needle);
/* Path A M165: owned string ops (byte index). Not &str borrow. */
int oo_str_starts_with(OoStr s, OoStr pre);
int oo_str_ends_with(OoStr s, OoStr suf);
long long oo_str_index_of(OoStr s, OoStr sub);
OoStr oo_str_repeat(OoStr s, long long n);

/* List API */
OoIList oo_ilist_new(void);
void oo_ilist_free(OoIList l);
OoIList oo_ilist_push(OoIList l, long long v);
long long oo_ilist_get(OoIList l, long long i);
long long oo_ilist_len(OoIList l);

OoSList oo_slist_new(void);
void oo_slist_free(OoSList l);
OoSList oo_slist_push(OoSList l, OoStr v);
OoStr oo_slist_get(OoSList l, long long i);
long long oo_slist_len(OoSList l);

/* Print API */
void oo_print_str(OoStr s);
void oo_eprint_str(OoStr s);
void oo_print_int(long long n);
void oo_print_bool(int b);
void oo_print_double(double x);
void oo_println(void);
void oo_eprintln(void);

/* Path A M166: IEEE-754 double trig/exp (Float type alias → double). No decimal. */
double oo_sin(double x);
double oo_cos(double x);
double oo_ln(double x);
double oo_exp(double x);
double oo_sqrt(double x);
double oo_pow(double base, double expn);

/* FS & System API */
OoResS oo_read_file(long long cap, OoStr path);
OoResV oo_write_file(long long cap, OoStr path, OoStr content);
int oo_path_exists(long long cap, OoStr path);
long long oo_file_size(long long cap, OoStr path);
OoResS oo_env_get(long long cap, OoStr key);
long long fs_file_size(long long cap, OoStr path);
OoSList fs_read_dir(long long cap, OoStr path);
int fs_is_dir(long long cap, OoStr path);

// sys capability functions
OoSList sys_args(long long cap);

/* Process-local caps (chs_rt_sys.c) */
long long oo_cap_grant_fs(void);
long long oo_cap_grant_sys(void);
long long oo_cap_grant_env(void);
long long oo_cap_grant_net(void);
long long oo_cap_grant_time(void);
long long oo_cap_grant_rand(void);
long long oo_cap_grant_alloc(void);
long long oo_cap_grant_ffi(void);
long long oo_cap_grant_audit(void);
long long oo_cap_grant_sign(void);
long long oo_cap_grant_hitl(void);
long long oo_cap_grant_process(void);
long long oo_cap_grant_sync(void);
long long oo_cap_grant_mem(void);
long long oo_cap_grant_http(void);
long long oo_cap_grant_tcp(void);
long long oo_cap_grant_udp(void);
long long oo_cap_grant_bind(void);
long long oo_cap_grant_audio(void);
long long oo_cap_grant_camera(void);
long long oo_cap_grant_usb(void);
long long oo_cap_grant_hid(void);
long long oo_cap_grant_window(void);
long long oo_cap_grant_frame(void);
long long oo_cap_grant_fsread(void);
long long oo_cap_grant_fswrite(void);

void oo_cap_require(long long got, long long want, const char *op);
void oo_cap_require_fs(long long got, const char *op);
void oo_cap_require_sys(long long got, const char *op);
void oo_cap_require_env(long long got, const char *op);
void oo_cap_require_net(long long got, const char *op);
void oo_cap_require_time(long long got, const char *op);
void oo_cap_require_rand(long long got, const char *op);
void oo_cap_require_alloc(long long got, const char *op);
void oo_cap_require_ffi(long long got, const char *op);
void oo_cap_require_audit(long long got, const char *op);
void oo_cap_require_sign(long long got, const char *op);
void oo_cap_require_hitl(long long got, const char *op);
void oo_cap_require_process(long long got, const char *op);
void oo_cap_require_sync(long long got, const char *op);
void oo_cap_require_mem(long long got, const char *op);
void oo_cap_require_http(long long got, const char *op);
void oo_cap_require_tcp(long long got, const char *op);
void oo_cap_require_udp(long long got, const char *op);
void oo_cap_require_bind(long long got, const char *op);
void oo_cap_require_audio(long long got, const char *op);
void oo_cap_require_camera(long long got, const char *op);
void oo_cap_require_usb(long long got, const char *op);
void oo_cap_require_hid(long long got, const char *op);
void oo_cap_require_window(long long got, const char *op);
void oo_cap_require_frame(long long got, const char *op);
void oo_cap_require_fsread(long long got, const char *op);
void oo_cap_require_fswrite(long long got, const char *op);

OoResS oo_dlopen(long long cap, OoStr path);
OoStr oo_host_ast_dump(long long cap, OoStr path);
OoStr oo_host_check(long long cap, OoStr path);
OoStr oo_host_token_dump(long long cap, OoStr path);
OoResS oo_chs_build(long long cap, OoStr src, OoStr out_bin);
OoResS oo_dlsym(long long cap, OoStr handle, OoStr name);
OoResS oo_dlclose(long long cap, OoStr handle);
long long oo_cap_grant_thread(void);
long long oo_cap_grant_gpu(void);
void oo_cap_require_thread(long long got, const char *op);
void oo_cap_require_gpu(long long got, const char *op);
OoStr crypto_md5_internal(OoStr data);
OoStr crypto_sha1_internal(OoStr data);
OoStr crypto_aes_encrypt_internal(OoStr key, OoStr plain);
OoStr crypto_hmac_sha256_internal(OoStr key, OoStr msg);
long long oo_byte_at(OoStr s, long long idx);
long long oo_bytes_len(OoStr s);
OoStr oo_byte_slice(OoStr s, long long start, long long end);
int oo_bytes_eq(OoStr a, OoStr b);
/* Path A Byte buffer: List[Int] 0..255 (not true List[Byte] ABI / not &str). */
OoStr oo_bytes_from_str(OoStr s);
OoStr oo_bytes_concat(OoStr a, OoStr b);
OoIList oo_bytes_new(void);
OoIList oo_bytes_push(OoIList l, long long b);
long long oo_bytes_get(OoIList l, long long i);
OoStr oo_bytes_to_str(OoIList l);
OoResS oo_tcp_bind(long long cap, long long port);
OoResS oo_tcp_connect(long long cap, OoStr host, long long port);
OoResS oo_bind_udp(long long cap, long long port);
OoResS oo_tcp_write(long long cap, long long slot, OoStr data);
OoResS oo_tcp_read(long long cap, long long slot, long long max_n);
OoResS oo_udp_recv(long long cap, long long slot, long long max_n);
OoResS oo_tcp_close(long long cap, long long slot);
OoResS oo_sock_raw(long long cap, long long proto);
OoResS oo_tls_connect(long long cap, OoStr host, long long port);
OoResS oo_sys_spawn(long long cap, OoStr cmd);
OoResS oo_sys_wait(long long cap, long long pid);
OoResS oo_sys_kill(long long cap, long long pid, long long sig);
/* M166 path A: OS syscall seals under SysCap — residual Err (not full async I/O) */
OoResS oo_sys_epoll_create(long long cap, long long flags);
OoResS oo_sys_inotify_init(long long cap);
OoResS oo_sys_prctl(long long cap, long long option);
OoResS oo_mutex_lock(long long cap, long long mid);
OoResS oo_mutex_unlock(long long cap, long long mid);
OoResS oo_thread_spawn(long long cap, OoStr name);
OoResS oo_thread_join(long long cap, long long slot);
OoResS oo_thread_join_s(long long cap, OoStr tid);
OoResS oo_channel_new(long long cap);
OoResS oo_channel_send(long long cap, long long slot, OoStr msg);
OoResS oo_channel_recv(long long cap, long long slot);
/* M165 path A: thin actors under ThreadCap + HITL free builtin */
OoResS oo_actor_spawn(long long cap, OoStr name);
OoResS oo_actor_send(long long cap, long long id, OoStr msg);
OoResS oo_actor_recv(long long cap, long long id);
/* Process-policy env: only OODA_* / OO_* keys (not product env_get). */
const char *oo_process_policy_getenv(const char *key);
/* Path-A metamorphic floor: process-local epoch / mix (not runtime code mutation). */
long long oo_meta_epoch(void);
long long oo_meta_mix(long long salt);
int oo_meta_is_path_a(void);
void oo_meta_decoy_touch(void);
/* HITL requires process EnvCap + FsCap (TTY / policy env). */
OoResS oo_verify_human(long long env, long long fs, OoStr msg);
OoResS oo_gpu_launch(long long cap, OoStr shader);
OoResS oo_sys_exec(long long cap, int argc, OoStr *argv);
OoResS oo_sys_exec1(long long cap, OoStr cmd);
OoResS oo_fetch(long long cap, OoStr url);
long long oo_now_ms(long long cap);
void oo_sleep_ms(long long cap, long long ms);
long long oo_random(long long cap);
void oo_seed(long long cap, long long s);
long long oo_alloc_bytes(long long cap, long long n);
void oo_free_bytes(long long cap, long long p);
long long oo_cg_sign(long long cap);
int oo_cg_verify(long long cap, long long sig);

OoSList str_split(OoStr s, OoStr delim);
OoStr str_trim(OoStr s);
OoSList fs_read_dir(long long cap, OoStr path);

#endif
