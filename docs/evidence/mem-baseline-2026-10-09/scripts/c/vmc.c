#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <mach/mach.h>
#include <mach/mach_vm.h>
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
  for (size_t len = 1 << 20; len <= (64u << 20); len <<= 1) {
    char *p = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
    memset(p, 7, len);
    double f0 = fp();
    char *q = mmap(0, 2 * len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
    double t0 = now();
    kern_return_t r = mach_vm_copy(mach_task_self(), (mach_vm_address_t)p, len, (mach_vm_address_t)q);
    double t1 = now();
    double f1 = fp();
    munmap(p, len);
    double f2 = fp();
    double t2 = now();
    memset(q, 9, len); // write all copied pages (COW faults?)
    double t3 = now();
    double f3 = fp();
    memset(q + len, 9, len);
    double f4 = fp();
    // baseline: memcpy
    char *s = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
    memset(s, 7, len);
    char *d = mmap(0, 2 * len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
    double t4 = now();
    memcpy(d, s, len);
    double t5 = now();
    printf("len %5zu MB: vm_copy r=%d %.3f ms fp %.1f->%.1f, after unmap src %.1f, rewrite %.3f ms fp %.1f, tail %.1f | memcpy %.3f ms\n",
           len >> 20, r, (t1 - t0) * 1e3, f0, f1, f2, (t3 - t2) * 1e3, f3, f4, (t5 - t4) * 1e3);
    munmap(q, 2 * len);
    munmap(s, len);
    munmap(d, 2 * len);
  }
  return 0;
}
