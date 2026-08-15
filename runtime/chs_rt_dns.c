#include "chs_rt.h"
#include "chs_rt_dns.h"
#include <netdb.h>
#include <arpa/inet.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <string.h>

OoResS oo_dns_resolve(long long cap, OoStr host) {
  oo_cap_require_net(cap, "dns_resolve");
  if (!host.data || host.len <= 0) {
    return (OoResS){0, oo_str_lit("ERR\tdns\tempty host name")};
  }
  char hbuf[256];
  if ((size_t)host.len >= sizeof hbuf) {
    return (OoResS){0, oo_str_lit("ERR\tdns\thost name too long")};
  }
  memcpy(hbuf, host.data, (size_t)host.len);
  hbuf[host.len] = '\0';

  if (strcmp(hbuf, "localhost") == 0) {
    return (OoResS){1, oo_str_lit("127.0.0.1")};
  }

  struct addrinfo hints, *res = NULL;
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_UNSPEC;
  hints.ai_socktype = SOCK_STREAM;

  int rc = getaddrinfo(hbuf, NULL, &hints, &res);
  if (rc != 0 || !res) {
    return (OoResS){0, oo_str_lit("ERR\tdns\thost lookup failed")};
  }

  char ip_buf[INET6_ADDRSTRLEN];
  memset(ip_buf, 0, sizeof ip_buf);
  if (res->ai_family == AF_INET) {
    struct sockaddr_in *sin = (struct sockaddr_in *)res->ai_addr;
    inet_ntop(AF_INET, &sin->sin_addr, ip_buf, sizeof ip_buf);
  } else if (res->ai_family == AF_INET6) {
    struct sockaddr_in6 *sin6 = (struct sockaddr_in6 *)res->ai_addr;
    inet_ntop(AF_INET6, &sin6->sin6_addr, ip_buf, sizeof ip_buf);
  } else {
    freeaddrinfo(res);
    return (OoResS){0, oo_str_lit("ERR\tdns\tunsupported address family")};
  }

  freeaddrinfo(res);
  return (OoResS){1, oo_str_lit(ip_buf)};
}

OoResS oo_dns_resolve_ipv4(long long cap, OoStr host) {
  oo_cap_require_net(cap, "dns_resolve_ipv4");
  if (!host.data || host.len <= 0) {
    return (OoResS){0, oo_str_lit("ERR\tdns\tempty host name")};
  }
  char hbuf[256];
  if ((size_t)host.len >= sizeof hbuf) {
    return (OoResS){0, oo_str_lit("ERR\tdns\thost name too long")};
  }
  memcpy(hbuf, host.data, (size_t)host.len);
  hbuf[host.len] = '\0';

  if (strcmp(hbuf, "localhost") == 0) {
    return (OoResS){1, oo_str_lit("127.0.0.1")};
  }

  struct addrinfo hints, *res = NULL;
  memset(&hints, 0, sizeof hints);
  hints.ai_family = AF_INET;
  hints.ai_socktype = SOCK_STREAM;

  int rc = getaddrinfo(hbuf, NULL, &hints, &res);
  if (rc != 0 || !res) {
    return (OoResS){0, oo_str_lit("ERR\tdns\tIPv4 lookup failed")};
  }

  char ip_buf[INET_ADDRSTRLEN];
  memset(ip_buf, 0, sizeof ip_buf);
  struct sockaddr_in *sin = (struct sockaddr_in *)res->ai_addr;
  inet_ntop(AF_INET, &sin->sin_addr, ip_buf, sizeof ip_buf);

  freeaddrinfo(res);
  return (OoResS){1, oo_str_lit(ip_buf)};
}
