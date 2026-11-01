/* zb DIR N: deflate DIR/i.raw (i < N; zlib_identity.py --dump) at level 9
   as writezip.c does (deflateInit(9): windowBits 15, memLevel 8), 20 passes,
   and report the time per pass and whether every output equals DIR/i.z (the
   bytes pdfTeX's zlib wrote). Build against TeX Live's zlib or another one
   (zbuild.sh). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "zlib.h"

static unsigned char *slurp(const char *p, size_t *n) {
  FILE *f = fopen(p, "rb");
  if (!f) return NULL;
  fseek(f, 0, SEEK_END);
  *n = (size_t)ftell(f);
  fseek(f, 0, SEEK_SET);
  unsigned char *b = malloc(*n + 1);
  if (fread(b, 1, *n, f) != *n) { fclose(f); free(b); return NULL; }
  fclose(f);
  return b;
}

int main(int argc, char **argv) {
  if (argc < 3) { fprintf(stderr, "usage: zb DIR N\n"); return 2; }
  int n = atoi(argv[2]), same = 0, diff = 0;
  char p[4096];
  double secs = 0;
  size_t in = 0;
  unsigned char *out = malloc(64u << 20);
  for (int rep = 0; rep < 20; rep++)
    for (int i = 0; i < n; i++) {
      size_t rn, zn;
      snprintf(p, sizeof p, "%s/%d.raw", argv[1], i);
      unsigned char *raw = slurp(p, &rn);
      if (!raw) continue;
      snprintf(p, sizeof p, "%s/%d.z", argv[1], i);
      unsigned char *z = slurp(p, &zn);
      z_stream s;
      memset(&s, 0, sizeof s);
      deflateInit(&s, 9);
      s.next_in = raw;
      s.avail_in = (uInt)rn;
      s.next_out = out;
      s.avail_out = 64u << 20;
      struct timespec t0, t1;
      clock_gettime(CLOCK_MONOTONIC, &t0);
      deflate(&s, Z_FINISH);
      clock_gettime(CLOCK_MONOTONIC, &t1);
      secs += (double)(t1.tv_sec - t0.tv_sec) + (double)(t1.tv_nsec - t0.tv_nsec) / 1e9;
      if (rep == 0) {
        if (s.total_out == zn && !memcmp(out, z, zn)) same++; else diff++;
        in += rn;
      }
      deflateEnd(&s);
      free(raw);
      free(z);
    }
  printf("zlib %s: %d identical, %d different; %zu bytes in; %.1f ms per pass\n", zlibVersion(), same,
         diff, in, secs * 1000 / 20);
  return 0;
}
