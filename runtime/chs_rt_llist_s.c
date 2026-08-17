/* Nested List[List[String]] — OoSList elements. */

static void oo_ll_S_retain(OoLL_S l) {
  if (!l.data) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  uint32_t rc = __atomic_load_n(&hdr->ref_count, __ATOMIC_ACQUIRE);
  uint32_t fl = __atomic_load_n(&hdr->flags, __ATOMIC_ACQUIRE);
  if (rc == 0 || rc == UINT32_MAX || (fl & 1)) return;
  while (rc > 0 && rc < UINT32_MAX) {
    if (__atomic_compare_exchange_n(&hdr->ref_count, &rc, rc + 1, 1,
                                    __ATOMIC_ACQ_REL, __ATOMIC_RELAXED)) {
      return;
    }
    rc = __atomic_load_n(&hdr->ref_count, __ATOMIC_RELAXED);
    fl = __atomic_load_n(&hdr->flags, __ATOMIC_RELAXED);
    if (rc == 0 || rc == UINT32_MAX || (fl & 1)) return;
  }
}

void oo_ll_S_release(OoLL_S l) {
  if (!oo_list_hdr_ok(l.data, l.len, l.cap)) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  uint32_t prev = __atomic_fetch_sub(&hdr->ref_count, 1, __ATOMIC_ACQ_REL);
  if (prev == 1) {
    for (long long i = 0; i < l.len; i++) {
      oo_slist_release(l.data[i]);
    }
    __atomic_store_n(&hdr->flags, 0xFFFFFFFFu, __ATOMIC_RELEASE);
    __atomic_thread_fence(__ATOMIC_RELEASE);
    pthread_mutex_lock(&g_quota_mu);
    oo_list_ambient_bytes -= (long long)(sizeof(OoListHeader) + l.cap * sizeof(OoSList));
    pthread_mutex_unlock(&g_quota_mu);
    free(hdr);
  }
}

OoLL_S oo_ll_S_new(void) {
  OoLL_S l = {NULL, 0, 0};
  return l;
}

OoLL_S oo_ll_S_push(OoLL_S l, OoSList v) {
  OoLL_S n;
  long long ncap = l.cap ? l.cap : 8;
  if (l.data && l.len < l.cap && oo_list_owned(l.data)) {
    l.data[l.len] = v;
    oo_slist_retain(v);
    l.len = l.len + 1;
    oo_ll_S_retain(l);
    return l;
  }
  while (ncap < l.len + 1) ncap *= 2;
  n.data = (OoSList *)oo_list_alloc_payload(sizeof(OoSList), (size_t)ncap);
  if (l.data && l.len > 0) {
    memcpy(n.data, l.data, (size_t)l.len * sizeof(OoSList));
    for (long long i = 0; i < l.len; i++) {
      oo_slist_retain(n.data[i]);
    }
  }
  n.data[l.len] = v;
  oo_slist_retain(v);
  n.len = l.len + 1;
  n.cap = ncap;
  {
    OoListHeader *hdr = ((OoListHeader *)n.data) - 1;
    __atomic_store_n(&hdr->ref_count, 1, __ATOMIC_RELEASE);
  }
  return n;
}

OoSList oo_ll_S_get(OoLL_S l, long long i) {
  OoSList r;
  oo_ll_S_retain(l);
  if (i < 0 || i >= l.len) {
    oo_ll_S_release(l);
    fprintf(stderr, "ERR\tll_S_get OOB\n");
    exit(1);
  }
  oo_slist_retain(l.data[i]);
  r = l.data[i];
  oo_ll_S_release(l);
  return r;
}

long long oo_ll_S_len(OoLL_S l) { return l.len; }
