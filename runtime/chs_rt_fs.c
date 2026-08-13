#include "chs_rt.h"
#include <limits.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <dirent.h>
#include <sys/stat.h>
void oo_cap_require_fsread(long long got, const char *op);
void oo_cap_require_fswrite(long long got, const char *op);
void oo_cap_require_env(long long got, const char *op);
static const char *fs_split_parent(const char *p, char *out, size_t sz) {
if(!p||p[0]!='/'||!out||sz<2)return NULL;
const char *s=strrchr(p,'/'); if(!s)return NULL;
if(s==p){ if(p[1]=='\0')return NULL; out[0]='/'; out[1]='\0'; return p+1; }
size_t n=(size_t)(s-p); if(n+1>sz)return NULL;
memcpy(out,p,n); out[n]='\0';
return s[1]=='\0'?NULL:s+1;
}
static void to_cpath(OoStr p, char* c, int max) {
long long n = p.len >= max ? max - 1 : p.len;
memcpy(c, p.data ? p.data : "", n); c[n] = '\0';
}
static int path_under_writedir(const char *path, const char *dir) {
char rp[PATH_MAX], rd[PATH_MAX], par[PATH_MAX];
if(!path||!dir||path[0]!='/'||dir[0]!='/'||!strcmp(dir,"/")||!realpath(dir,rd))return 0;
size_t n = strlen(rd); if(!n)return 0;
if (realpath(path, rp)) return !strncmp(rp,rd,n) && (rp[n]=='\0'||rp[n]=='/');
const char *b = fs_split_parent(path, par, PATH_MAX);
if(!b||!b[0]||!strcmp(b,".")||!strcmp(b,"..")||strchr(b,'/')||!realpath(par,rp))return 0;
return !strncmp(rp,rd,n) && (rp[n]=='\0'||rp[n]=='/');
}
static int writedir_open_trunc(const char *path) {
char par[PATH_MAX], rp[PATH_MAX];
const char *b = fs_split_parent(path, par, PATH_MAX);
if(!b||!b[0]||!strcmp(b,".")||!strcmp(b,"..")||!realpath(par,rp))return -1;
int dfd = open(rp, O_RDONLY|O_DIRECTORY|O_CLOEXEC); if(dfd<0)return -1;
int fd = openat(dfd, b, O_WRONLY|O_CREAT|O_TRUNC|O_CLOEXEC|O_NOFOLLOW, 0666);
close(dfd); return fd;
}
OoResS oo_read_file(long long cap, OoStr path) {
oo_cap_require_fsread(cap, "read_file"); OoResS r={0, oo_str_lit("read_file failed")};
FILE *f = fopen(path.data, "rb"); if (!f) return r;
if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return r; }
long sz = ftell(f); if (sz < 0) { fclose(f); return r; }
if (fseek(f, 0, SEEK_SET) != 0) { fclose(f); return r; }
char *buf = oo_str_alloc_payload((size_t)sz);
size_t n = fread(buf, 1, (size_t)sz, f);
if (ferror(f)) { oo_str_release((OoStr){buf, (long long)n}); fclose(f); return r; }
buf[n] = 0; fclose(f);
r.ok = 1; r.val.data = buf; r.val.len = (long long)n;
return r;
}
OoResV oo_write_file(long long cap, OoStr path, OoStr content) {
oo_cap_require_fswrite(cap, "write_file"); OoResV r={0, oo_str_lit("write_file failed")};
char cpath[PATH_MAX]; to_cpath(path, cpath, PATH_MAX);
const char *dir = oo_process_policy_getenv("OODA_FS_WRITEDIR");
if (!dir || !dir[0] || !path_under_writedir(cpath, dir)) {
r.err = oo_str_lit("write_file denied: path not under OODA_FS_WRITEDIR"); return r; }
int fd = writedir_open_trunc(cpath); if (fd < 0) return r;
FILE *f = fdopen(fd, "wb"); if (!f) { close(fd); return r; }
size_t want = content.data ? (size_t)content.len : 0;
int bad = (want && fwrite(content.data, 1, want, f) != want) || ferror(f);
if (fclose(f) != 0) bad = 1;
if (!bad) { r.ok = 1; r.err = oo_str_lit(""); }
return r;
}
int oo_path_exists(long long cap, OoStr path) {
oo_cap_require_fsread(cap, "path_exists");
FILE *f=fopen(path.data,"rb");
if(f){fclose(f);return 1;}
return 0;
}
long long oo_file_size(long long cap, OoStr path) {
oo_cap_require_fsread(cap, "file_size");
FILE *f=fopen(path.data,"rb"); if(!f)return -1;
fseek(f,0,SEEK_END); long long sz=ftell(f); fclose(f);
return sz;
}
OoResS oo_env_get(long long cap, OoStr key) {
oo_cap_require_env(cap, "env_get");
OoResS r;
const char *val = oo_process_policy_getenv(key.data ? key.data : "");
if (val) {
r.ok = 1;
r.val = oo_str_lit(val);
} else {
r.ok = 0;
r.val = oo_str_lit("env var not set");
}
return r;
}
long long oo_monotonic_us(void) {
struct timespec ts;
clock_gettime(CLOCK_MONOTONIC, &ts);
long long us = (long long)ts.tv_sec * 1000000LL + (long long)ts.tv_nsec / 1000LL;
return us > 0LL ? us : 1LL;
}
OoSList oo_fs_read_dir(long long cap, OoStr path) {
oo_cap_require_fsread(cap, "fs_read_dir");
OoSList l = oo_slist_new();
const char *p = path.data ? path.data : "";
DIR *d = opendir(p);
if (!d) return l;
struct dirent *dir;
while ((dir = readdir(d)) != NULL) {
if (strcmp(dir->d_name, ".") == 0 || strcmp(dir->d_name, "..") == 0) continue;
OoStr part = oo_str_lit(dir->d_name);
OoSList next = oo_slist_push(l, part);
oo_slist_release(l);
l = next;
oo_str_release(part);
}
closedir(d);
return l;
}
int oo_fs_is_dir(long long cap, OoStr path) {
oo_cap_require_fsread(cap, "fs_is_dir");
char cpath[1024]; to_cpath(path, cpath, 1024);
struct stat st;
return (stat(cpath, &st) == 0 && S_ISDIR(st.st_mode)) ? 1 : 0;
}
OoResV oo_fs_remove_file(long long cap, OoStr path) {
oo_cap_require_fswrite(cap, "fs_remove_file");
char cpath[PATH_MAX]; to_cpath(path, cpath, PATH_MAX);
OoResV r={0, oo_str_lit("fs_remove_file failed")};
const char *dir = oo_process_policy_getenv("OODA_FS_WRITEDIR");
if (!dir || !dir[0] || !path_under_writedir(cpath, dir)) {
r.err = oo_str_lit("fs_remove_file denied: path not under OODA_FS_WRITEDIR"); return r; }
if (unlink(cpath) == 0) { r.ok = 1; r.err = oo_str_lit(""); }
return r;
}
OoResV oo_fs_mkdir(long long cap, OoStr path) {
oo_cap_require_fswrite(cap, "fs_mkdir");
char cpath[PATH_MAX]; to_cpath(path, cpath, PATH_MAX);
OoResV r={0, oo_str_lit("fs_mkdir failed")};
const char *dir = oo_process_policy_getenv("OODA_FS_WRITEDIR");
if (!dir || !dir[0] || !path_under_writedir(cpath, dir)) {
r.err = oo_str_lit("fs_mkdir denied: path not under OODA_FS_WRITEDIR"); return r; }
if (mkdir(cpath, 0777) == 0) { r.ok = 1; r.err = oo_str_lit(""); }
return r;
}
OoResV oo_fs_hardlink(long long cap, OoStr oldpath, OoStr newpath) {
oo_cap_require_fswrite(cap, "fs_hardlink");
char cold[PATH_MAX], cnew[PATH_MAX];
to_cpath(oldpath, cold, PATH_MAX); to_cpath(newpath, cnew, PATH_MAX);
OoResV r={0,oo_str_lit("fs_hardlink failed")}; const char *dir = oo_process_policy_getenv("OODA_FS_WRITEDIR");
if (!dir || !dir[0] || !path_under_writedir(cnew, dir) || !path_under_writedir(cold, dir)) { r.err=oo_str_lit("fs_hardlink denied"); return r; }
if (link(cold, cnew) == 0) { r.ok = 1; r.err = oo_str_lit(""); }
return r;
}
OoResV oo_fs_symlink(long long cap, OoStr target, OoStr linkpath) {
oo_cap_require_fswrite(cap, "fs_symlink");
char ctarget[PATH_MAX], clink[PATH_MAX];
to_cpath(target, ctarget, PATH_MAX); to_cpath(linkpath, clink, PATH_MAX);
OoResV r={0,oo_str_lit("fs_symlink failed")}; const char *dir = oo_process_policy_getenv("OODA_FS_WRITEDIR");
if (!dir || !dir[0] || !path_under_writedir(clink, dir) || !path_under_writedir(ctarget, dir)) { r.err=oo_str_lit("fs_symlink denied"); return r; }
if (symlink(ctarget, clink) == 0) { r.ok = 1; r.err = oo_str_lit(""); }
return r;
}
