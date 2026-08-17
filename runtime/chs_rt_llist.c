/* Nested List[List[Int]] — same header/quota as OoIList, OoIList elements. */

static void oo_ll_I_retain(OoLL_I l) {
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

void oo_ll_I_release(OoLL_I l) {
  if (!oo_list_hdr_ok(l.data, l.len, l.cap)) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  uint32_t prev = __atomic_fetch_sub(&hdr->ref_count, 1, __ATOMIC_ACQ_REL);
  if (prev == 1) {
    for (long long i = 0; i < l.len; i++) {
      oo_ilist_release(l.data[i]);
    }
    __atomic_store_n(&hdr->flags, 0xFFFFFFFFu, __ATOMIC_RELEASE);
    __atomic_thread_fence(__ATOMIC_RELEASE);
    pthread_mutex_lock(&g_quota_mu);
    oo_list_ambient_bytes -= (long long)(sizeof(OoListHeader) + l.cap * sizeof(OoIList));
    pthread_mutex_unlock(&g_quota_mu);
    free(hdr);
  }
}

OoLL_I oo_ll_I_new(void) {
  OoLL_I l = {NULL, 0, 0};
  return l;
}

OoLL_I oo_ll_I_push(OoLL_I l, OoIList v) {
  OoLL_I n;
  long long ncap = l.cap ? l.cap : 8;
  if (l.data && l.len < l.cap && oo_list_owned(l.data)) {
    l.data[l.len] = v;
    oo_ilist_retain(v);
    l.len = l.len + 1;
    oo_ll_I_retain(l);
    return l;
  }
  while (ncap < l.len + 1) ncap *= 2;
  n.data = (OoIList *)oo_list_alloc_payload(sizeof(OoIList), (size_t)ncap);
  if (l.data && l.len > 0) {
    memcpy(n.data, l.data, (size_t)l.len * sizeof(OoIList));
    for (long long i = 0; i < l.len; i++) {
      oo_ilist_retain(n.data[i]);
    }
  }
  n.data[l.len] = v;
  oo_ilist_retain(v);
  n.len = l.len + 1;
  n.cap = ncap;
  {
    OoListHeader *hdr = ((OoListHeader *)n.data) - 1;
    __atomic_store_n(&hdr->ref_count, 1, __ATOMIC_RELEASE);
  }
  return n;
}

OoIList oo_ll_I_get(OoLL_I l, long long i) {
  OoIList r;
  oo_ll_I_retain(l);
  if (i < 0 || i >= l.len) {
    oo_ll_I_release(l);
    fprintf(stderr, "ERR\tll_I_get OOB\n");
    exit(1);
  }
  oo_ilist_retain(l.data[i]);
  r = l.data[i];
  oo_ll_I_release(l);
  return r;
}

long long oo_ll_I_len(OoLL_I l) { return l.len; }
