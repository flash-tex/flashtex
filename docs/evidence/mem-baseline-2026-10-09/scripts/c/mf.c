#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <mach/mach.h>
static double fp(void) {
  task_vm_info_data_t i;
  mach_msg_type_number_t c = TASK_VM_INFO_COUNT;
  task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&i, &c);
  return i.phys_footprint / 1048576.0;
}
int main(void) {
  size_t len = 64 << 20;
  const char *names[] = {"MADV_FREE", "MADV_DONTNEED"};
  int adv[] = {MADV_FREE, MADV_DONTNEED};
  for (int k = 0; k < 2; k++) {
    char *a = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
    memset(a, 1, len);
    double f0 = fp();
    int r = madvise(a, len, adv[k]);
    printf("%s r=%d %.1f -> %.1f\n", names[k], r, f0, fp());
    munmap(a, len);
  }
  return 0;
}
