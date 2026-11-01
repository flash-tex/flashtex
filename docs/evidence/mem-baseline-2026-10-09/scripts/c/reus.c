#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <mach/mach.h>
#include <time.h>
static double fp(void) {
  task_vm_info_data_t i;
  mach_msg_type_number_t c = TASK_VM_INFO_COUNT;
  task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&i, &c);
  return i.phys_footprint / 1048576.0;
}
static double now(void) {
  struct timespec t;
  clock_gettime(CLOCK_MONOTONIC, &t);
  return t.tv_sec + t.tv_nsec / 1e9;
}
int main(void) {
  size_t len = 64 << 20;
  char *p = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
  double t0 = now();
  memset(p, 1, len);
  double t1 = now();
  printf("first touch %.2f ms, fp %.1f\n", (t1 - t0) * 1e3, fp());
  t0 = now();
  memset(p, 2, len);
  t1 = now();
  printf("rewrite (resident) %.2f ms\n", (t1 - t0) * 1e3);
  t0 = now();
  int r = madvise(p, len, MADV_FREE_REUSABLE);
  t1 = now();
  printf("REUSABLE r=%d %.3f ms, fp %.1f\n", r, (t1 - t0) * 1e3, fp());
  t0 = now();
  r = madvise(p, len, MADV_FREE_REUSE);
  t1 = now();
  printf("REUSE r=%d %.3f ms, fp %.1f\n", r, (t1 - t0) * 1e3, fp());
  t0 = now();
  memset(p, 3, len);
  t1 = now();
  printf("write after reuse %.2f ms, fp %.1f, p[5]=%d\n", (t1 - t0) * 1e3, fp(), p[5]);
  // munmap + fresh map cost
  munmap(p, len);
  p = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
  t0 = now();
  memset(p, 1, len);
  t1 = now();
  printf("fresh map touch %.2f ms\n", (t1 - t0) * 1e3);
  // 1 MB blocks: REUSABLE+REUSE per block cost
  t0 = now();
  for (int k = 0; k < 64; k++) {
    madvise(p + ((size_t)k << 20), 1 << 20, MADV_FREE_REUSABLE);
    madvise(p + ((size_t)k << 20), 1 << 20, MADV_FREE_REUSE);
  }
  t1 = now();
  printf("64 x (REUSABLE+REUSE) of 1 MB: %.3f ms\n", (t1 - t0) * 1e3);
  t0 = now();
  memset(p, 4, len);
  t1 = now();
  printf("write after those %.2f ms fp %.1f\n", (t1 - t0) * 1e3, fp());
  return 0;
}
