/* Dedicated intern for 0..8191. pack_skip/pack_tok call to_string on
 * line, col, and byte index every lexer step. Hash-slice intern collides. */

#define OO_INT_INTERN 8192
static char g_int[OO_INT_INTERN][sizeof(OoStrHeader) + 6];
static unsigned char g_int_len[OO_INT_INTERN];
static int g_int_ok = 0;

static void g_int_init(void) {
    int n;
    if (g_int_ok) {
        return;
    }
    for (n = 0; n < OO_INT_INTERN; n++) {
        OoStrHeader *h = (OoStrHeader *)g_int[n];
        char *p = g_int[n] + sizeof(OoStrHeader);
        int w = snprintf(p, 6, "%d", n);
        h->ref_count = 1;
        h->flags = OO_FLAG_STATIC;
        g_int_len[n] = (unsigned char)(w > 0 ? w : 0);
    }
    g_int_ok = 1;
}

OoStr oo_int_intern(long long n) {
    OoStr r;
    if (n >= 0 && n < OO_INT_INTERN) {
        g_int_init();
        r.len = (long long)g_int_len[(int)n];
        r.data = g_int[(int)n] + sizeof(OoStrHeader);
        return r;
    }
    {
        char buf[32];
        int w = snprintf(buf, sizeof(buf), "%lld", n);
        if (w < 0) {
            abort();
        }
        return oo_str_intern_bytes(buf, (long long)w);
    }
}
