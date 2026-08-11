/* M163 TLS path A: NetCap + optional OpenSSL 1.2+ client; else residual.
 * Default fail-closed after real TCP connect. OO_HAVE_OPENSSL → handshake.
 * OODA_TLS_INSECURE_TCP=1 → Ok after TCP-only (insecure residual; honesty). */
#include "chs_rt.h"
#include <unistd.h>
#include <errno.h>
#include <netdb.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#if defined(OO_HAVE_OPENSSL)
#include <openssl/ssl.h>
#include <openssl/err.h>
#endif

static OoResS tls_err(const char *msg) {
  OoResS r;
  r.ok = 0;
  r.val = oo_str_lit(msg);
  return r;
}

/* TCP connect; returns fd >= 0 or -1. *err set on failure (static-ish msg). */
static int tls_tcp_fd(const char *h, long long port, const char **err) {
  char portstr[16];
  struct addrinfo hints, *res = NULL, *rp;
  int fd = -1;
  *err = "tls_connect: resolve failed";
  if (!h || !h[0] || port < 1 || port > 65535) {
    *err = "tls_connect: bad host/port";
    return -1;
  }
  snprintf(portstr, sizeof portstr, "%lld", (long long)port);
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_UNSPEC;
  hints.ai_socktype = SOCK_STREAM;
  if (getaddrinfo(h, portstr, &hints, &res) != 0) return -1;
  *err = "tls_connect: connection refused";
  for (rp = res; rp; rp = rp->ai_next) {
    fd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
    if (fd < 0) continue;
    if (connect(fd, rp->ai_addr, rp->ai_addrlen) == 0) break;
    close(fd);
    fd = -1;
  }
  freeaddrinfo(res);
  return fd;
}

#if defined(OO_HAVE_OPENSSL)
static OoResS tls_handshake_openssl(int fd, const char *h, long long port) {
  OoResS r;
  SSL_CTX *ctx = NULL;
  SSL *ssl = NULL;
  char buf[160];
  const SSL_METHOD *meth;
  int rc;
  meth = TLS_client_method();
  if (!meth) {
    close(fd);
    return tls_err("tls_connect: TLS_client_method failed");
  }
  ctx = SSL_CTX_new(meth);
  if (!ctx) {
    close(fd);
    return tls_err("tls_connect: SSL_CTX_new failed");
  }
  /* TLS 1.2+ only — no SSLv3/TLS1.0/1.1 product surface */
  SSL_CTX_set_min_proto_version(ctx, TLS1_2_VERSION);
  SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, NULL);
  SSL_CTX_set_default_verify_paths(ctx);
  ssl = SSL_new(ctx);
  if (!ssl) {
    SSL_CTX_free(ctx);
    close(fd);
    return tls_err("tls_connect: SSL_new failed");
  }
  SSL_set_fd(ssl, fd);
  if (h && h[0]) (void)SSL_set_tlsext_host_name(ssl, h);
  rc = SSL_connect(ssl);
  if (rc != 1) {
    SSL_free(ssl);
    SSL_CTX_free(ctx);
    close(fd);
    return tls_err("tls_connect: SSL_connect failed");
  }
  snprintf(buf, sizeof buf, "tls-connected:%s:%lld", h ? h : "", (long long)port);
  SSL_shutdown(ssl);
  SSL_free(ssl);
  SSL_CTX_free(ctx);
  close(fd);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}
#endif

OoResS oo_tls_connect(long long cap, OoStr host, long long port) {
  const char *h;
  const char *err = NULL;
  int fd;
  oo_cap_require_net(cap, "tls_connect");
  h = host.data ? host.data : "";
  fd = tls_tcp_fd(h, port, &err);
  if (fd < 0) return tls_err(err ? err : "tls_connect: connection refused");

#if defined(OO_HAVE_OPENSSL)
  return tls_handshake_openssl(fd, h, port);
#else
  /* Path A without OpenSSL: TCP proved; residual or explicit insecure TCP-only. */
  {
    OoResS r;
    const char *insec = getenv("OODA_TLS_INSECURE_TCP");
    char buf[160];
    if (insec && strcmp(insec, "1") == 0) {
      close(fd);
      snprintf(buf, sizeof buf,
               "insecure residual: TCP-only (no TLS) %s:%lld", h, (long long)port);
      r.ok = 1;
      r.val = oo_str_lit(buf);
      return r;
    }
    close(fd);
    return tls_err("tls residual: OpenSSL not linked");
  }
#endif
}
