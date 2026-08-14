/* One-char intern. oo_char_at used to heap-allocate every character.
 * check of oodac/main.oo leaked those payloads across 174 modules (~12 GiB).
 * STATIC flag makes retain/release no-ops (oo_str_hdr_ok rejects STATIC). */

static char g_ascii[256][sizeof(OoStrHeader) + 2];
static int g_ascii_ok = 0;

static void g_ascii_init(void) {
    int i;
    if (g_ascii_ok) {
        return;
    }
    for (i = 0; i < 256; i++) {
        OoStrHeader *h = (OoStrHeader *)g_ascii[i];
        h->ref_count = 1;
        h->flags = OO_FLAG_STATIC;
        g_ascii[i][sizeof(OoStrHeader)] = (char)i;
        g_ascii[i][sizeof(OoStrHeader) + 1] = 0;
    }
    g_ascii_ok = 1;
}

OoStr oo_str_ascii_intern(unsigned char c) {
    OoStr r;
    g_ascii_init();
    r.len = 1;
    r.data = g_ascii[c] + sizeof(OoStrHeader);
    return r;
}

#define OO_SLICE_SLOTS 4096
#define OO_SLICE_MAX 16
static char g_slice[OO_SLICE_SLOTS][sizeof(OoStrHeader) + OO_SLICE_MAX + 1];
static unsigned char g_slice_len[OO_SLICE_SLOTS];
static unsigned char g_slice_full[OO_SLICE_SLOTS];

OoStr oo_str_intern_bytes(const char *p, long long n) {
    unsigned h = 2166136261u;
    long long i;
    unsigned slot;
    OoStr r;
    if (n <= 0) {
        static char empty[sizeof(OoStrHeader) + 1];
        static int empty_ok = 0;
        OoStrHeader *eh;
        if (!empty_ok) {
            eh = (OoStrHeader *)empty;
            eh->ref_count = 1;
            eh->flags = OO_FLAG_STATIC;
            empty[sizeof(OoStrHeader)] = 0;
            empty_ok = 1;
        }
        r.len = 0;
        r.data = empty + sizeof(OoStrHeader);
        return r;
    }
    if (n == 1) {
        return oo_str_ascii_intern((unsigned char)p[0]);
    }
    if (n > OO_SLICE_MAX) {
        r.len = n;
        r.data = oo_str_alloc_payload((size_t)n);
        memcpy(r.data, p, (size_t)n);
        return r;
    }
    for (i = 0; i < n; i++) {
        h ^= (unsigned char)p[i];
        h *= 16777619u;
    }
    slot = h % OO_SLICE_SLOTS;
    if (g_slice_full[slot]
        && g_slice_len[slot] == (unsigned char)n
        && memcmp(g_slice[slot] + sizeof(OoStrHeader), p, (size_t)n) == 0) {
        r.len = n;
        r.data = g_slice[slot] + sizeof(OoStrHeader);
        return r;
    }
    if (!g_slice_full[slot]) {
        OoStrHeader *hdr = (OoStrHeader *)g_slice[slot];
        hdr->ref_count = 1;
        hdr->flags = OO_FLAG_STATIC;
        memcpy(g_slice[slot] + sizeof(OoStrHeader), p, (size_t)n);
        g_slice[slot][sizeof(OoStrHeader) + (size_t)n] = 0;
        g_slice_len[slot] = (unsigned char)n;
        g_slice_full[slot] = 1;
        r.len = n;
        r.data = g_slice[slot] + sizeof(OoStrHeader);
        return r;
    }
    r.len = n;
    r.data = oo_str_alloc_payload((size_t)n);
    memcpy(r.data, p, (size_t)n);
    return r;
}
