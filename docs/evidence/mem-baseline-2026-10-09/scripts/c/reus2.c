#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <mach/mach.h>
#include <mach/mach_vm.h>
static double fp(void) {
  task_vm_info_data_t i;
  mach_msg_type_number_t c = TASK_VM_INFO_COUNT;
  task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&i, &c);
  return i.phys_footprint / 1048576.0;
}
int main(void) {
  size_t len = 16 << 20;
  // 1. plain
  char *a = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
  memset(a, 1, len);
  printf("plain written %.1f\n", fp());
  int r = madvise(a, len, MADV_FREE_REUSABLE);
  printf("plain reusable r=%d %.1f\n", r, fp());
  madvise(a, len, MADV_FREE_REUSE);
  memset(a, 2, len);
  printf("plain reused+written %.1f\n", fp());
  // 2. split: reusable on a part
  r = madvise(a + (4 << 20), 8 << 20, MADV_FREE_REUSABLE);
  printf("part reusable r=%d %.1f\n", r, fp());
  munmap(a, len);
  // 3. vm_copy dest
  a = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
  memset(a, 1, len);
  char *b = mmap(0, 2 * len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
  mach_vm_copy(mach_task_self(), (mach_vm_address_t)a, len, (mach_vm_address_t)b);
  munmap(a, len);
  printf("copied %.1f\n", fp());
  memset(b, 3, 2 * len);
  printf("copy written %.1f\n", fp());
  r = madvise(b, 2 * len, MADV_FREE_REUSABLE);
  printf("copy reusable r=%d %.1f\n", r, fp());
  // 4. partially-unmapped tail then reusable
  char *c = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
  memset(c, 1, len);
  munmap(c + (8 << 20), 8 << 20);
  printf("c half %.1f\n", fp());
  r = madvise(c, 8 << 20, MADV_FREE_REUSABLE);
  printf("c reusable r=%d %.1f\n", r, fp());
  return 0;
}
