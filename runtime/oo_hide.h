/* Load-time hide table. Not chs_rt*.c (AGY RT lane).
 * Hide-off: zeros (stable). Hide-on (OODA_HIDE=1): process entropy.
 * Does not rewrite .text. Disk bytes unchanged. */
#ifndef OO_HIDE_H
#define OO_HIDE_H

#define OO_HIDE_SLOTS 16

int oo_hide_enabled(void);
int oo_hide_loaded_at_start(void);
int oo_hide_ready(void);
void oo_hide_table(unsigned long long *out, int n);
unsigned long long oo_hide_fingerprint(void);

#endif
