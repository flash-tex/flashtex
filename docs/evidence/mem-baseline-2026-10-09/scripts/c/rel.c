// rel.c N SIZE: footprint after allocating N blocks of SIZE, touching them,
// freeing them, then pressure relief; then a churn loop's time.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <malloc/malloc.h>
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
int main(int argc, char **argv) {
  size_t n = argc > 1 ? atol(argv[1]) : 1000, sz = argc > 2 ? atol(argv[2]) : 65536;
  void **p = malloc(n * sizeof(void *));
  double f0 = fp();
  for (size_t i = 0; i < n; i++) { p[i] = malloc(sz); memset(p[i], 1, sz); }
  double f1 = fp();
  for (size_t i = 0; i < n; i++) free(p[i]);
  double f2 = fp();
  size_t r = malloc_zone_pressure_relief(NULL, 0);
  double f3 = fp();
  // churn: 20 rounds of allocate-touch-free of the same set
  double t0 = now();
  for (int k = 0; k < 20; k++) {
    for (size_t i = 0; i < n; i++) { p[i] = malloc(sz); memset(p[i], k, sz); }
    for (size_t i = 0; i < n; i++) free(p[i]);
  }
  double t1 = now();
  double f4 = fp();
  printf("sz=%zu n=%zu start=%.1f alloc=%.1f free=%.1f relief(%zu)=%.1f churn_ms=%.1f after_churn=%.1f\n",
         sz, n, f0, f1, f2, r, f3, (t1 - t0) * 1e3, f4);
  return 0;
}
