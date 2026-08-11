/* M162: real TCP/UDP under NetCap. TLS lives in chs_rt_tls.c (M163). */
#include "chs_rt.h"
#include <unistd.h>
#include <errno.h>
#include <netdb.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <arpa/inet.h>
#include <netinet/in.h>

static OoResS net_err(const char *msg) {
  OoResS r;
  r.ok = 0;
  r.val = oo_str_lit(msg);
  return r;
}

OoResS oo_tcp_bind(long long cap, long long port) {
  OoResS r;
  int fd;
  struct sockaddr_in addr;
  char buf[64];
  oo_cap_require_net(cap, "tcp_bind");
  if (port < 1 || port > 65535) return net_err("tcp_bind: bad port");
  fd = socket(AF_INET, SOCK_STREAM, 0);
  if (fd < 0) return net_err("tcp_bind: socket failed");
  memset(&addr, 0, sizeof addr);
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  addr.sin_port = htons((uint16_t)port);
  {
    int yes = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof yes);
  }
  if (bind(fd, (struct sockaddr *)&addr, sizeof addr) != 0) {
    close(fd);
    return net_err("tcp_bind: bind failed");
  }
  if (listen(fd, 1) != 0) {
    close(fd);
    return net_err("tcp_bind: listen failed");
  }
  snprintf(buf, sizeof buf, "listen-fd:%d", fd);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  close(fd);
  return r;
}

OoResS oo_tcp_connect(long long cap, OoStr host, long long port) {
  OoResS r;
  char portstr[16];
  struct addrinfo hints, *res = NULL, *rp;
  int fd = -1;
  const char *h;
  char buf[128];
  oo_cap_require_net(cap, "tcp_connect");
  h = host.data ? host.data : "";
  if (!h[0] || port < 1 || port > 65535) return net_err("tcp_connect: bad host/port");
  snprintf(portstr, sizeof portstr, "%lld", (long long)port);
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_UNSPEC;
  hints.ai_socktype = SOCK_STREAM;
  if (getaddrinfo(h, portstr, &hints, &res) != 0) return net_err("tcp_connect: resolve failed");
  for (rp = res; rp; rp = rp->ai_next) {
    fd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
    if (fd < 0) continue;
    if (connect(fd, rp->ai_addr, rp->ai_addrlen) == 0) break;
    close(fd);
    fd = -1;
  }
  freeaddrinfo(res);
  if (fd < 0) return net_err("tcp_connect: connection refused");
  close(fd);
  snprintf(buf, sizeof buf, "connected:%s:%lld", h, (long long)port);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}

OoResS oo_bind_udp(long long cap, long long port) {
  OoResS r;
  int fd;
  struct sockaddr_in addr;
  char buf[64];
  oo_cap_require_net(cap, "bind_udp");
  if (port < 1 || port > 65535) return net_err("bind_udp: bad port");
  fd = socket(AF_INET, SOCK_DGRAM, 0);
  if (fd < 0) return net_err("bind_udp: socket failed");
  memset(&addr, 0, sizeof addr);
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  addr.sin_port = htons((uint16_t)port);
  if (bind(fd, (struct sockaddr *)&addr, sizeof addr) != 0) {
    close(fd);
    return net_err("bind_udp: bind failed");
  }
  snprintf(buf, sizeof buf, "udp-fd:%d", fd);
  close(fd);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}
