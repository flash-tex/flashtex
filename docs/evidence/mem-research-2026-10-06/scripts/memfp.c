// memfp.c - what each memory-release technique does to macOS phys_footprint (FlashTeX MEM-RESEARCH)
// Not run yet: a recipe for a team Mac (README "macOS: the OS view").
// build: clang -O2 -Wall -o memfp memfp.c
// usage: ./memfp CASE [MB]   HOLD=secs pauses at the interesting point (run footprint/vmmap then);
//        WAIT=secs for 'compressed'. One case per process.
#include <fcntl.h>
#include <libproc.h>
#include <mach/mach.h>
#include <mach/mach_time.h>
#include <mach/mach_vm.h>
#include <malloc/malloc.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

static size_t PG, LEN, NPG;
static mach_timebase_info_data_t TB;
#define MB(x) ((double)(x) / 1048576.0)
static uint64_t now(void) { return mach_absolute_time(); }
static double ms(uint64_t a, uint64_t b) { return (double)(b - a) * TB.numer / TB.denom / 1e6; }
static char mark(size_t i) { return (char)(i % 127 + 1); }

static void show(const char *what) {
    struct rusage_info_v4 ri;
    task_vm_info_data_t v;
    mach_msg_type_number_t n = TASK_VM_INFO_COUNT;
    if (proc_pid_rusage(getpid(), RUSAGE_INFO_V4, (rusage_info_t *)&ri) != 0) ri.ri_phys_footprint = 0;
    if (task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&v, &n) != KERN_SUCCESS) memset(&v, 0, sizeof v);
    printf("%-30s fp(rusage) %7.1f fp(tvi) %7.1f internal %7.1f compressed %7.1f external %7.1f "
           "reusable %7.1f purg_vol %7.1f purg_nonvol %7.1f decomp %d\n",
           what, MB(ri.ri_phys_footprint), MB(v.phys_footprint), MB(v.internal), MB(v.compressed),
           MB(v.external), MB(v.reusable), MB(v.ledger_purgeable_volatile),
           MB(v.ledger_purgeable_nonvolatile), v.decompressions);
    fflush(stdout);
}

static void hold(const char *when) {
    const char *h = getenv("HOLD");
    if (!h) return;
    printf("  HOLD %s s %s -- now run: footprint %d ; vmmap --summary %d\n", h, when, getpid(), getpid());
    fflush(stdout);
    sleep((unsigned)atoi(h));
    show("  after HOLD");
}

static char *anon(void) {
    char *p = mmap(NULL, LEN, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
    if (p == MAP_FAILED) { perror("mmap"); exit(1); }
    return p;
}

/* Dirty every page (marker in byte 0); with fill, half of each page pseudo-random (~2:1 compressible). */
static void touch(char *p, int fill) {
    uint64_t x = 88172645463325252ull;
    for (size_t i = 0; i < NPG; i++) {
        if (fill) {
            uint64_t *w = (uint64_t *)(p + i * PG);
            for (size_t j = 0; j < PG / 16; j++) { x ^= x << 13; x ^= x >> 7; x ^= x << 17; w[j] = x; }
        }
        p[i * PG] = mark(i);
    }
}

/* Read byte 0 of each page, then write it: refault time per MB, and pages whose contents survived. */
static void retouch(char *p, const char *what) {
    size_t kept = 0;
    uint64_t t0 = now();
    for (size_t i = 0; i < NPG; i++) {
        kept += ((volatile char *)p)[i * PG] == mark(i);
        p[i * PG] = mark(i);
    }
    double t = ms(t0, now());
    printf("  %-28s %8.2f ms  %.4f ms/MB  contents kept %zu/%zu pages\n", what, t, t / MB(LEN), kept, NPG);
    show("after re-touch");
}

static int mkfile(char *path) {
    static char buf[1 << 20];
    snprintf(path, 1024, "%s/memfp.%d.bin", getenv("TMPDIR") ? getenv("TMPDIR") : "/tmp", getpid());
    int fd = open(path, O_RDWR | O_CREAT | O_TRUNC, 0600);
    if (fd < 0) { perror(path); exit(1); }
    memset(buf, 0x5a, sizeof buf);
    for (size_t i = 0; i < LEN; i += sizeof buf)
        if (write(fd, buf, sizeof buf) != (ssize_t)sizeof buf) { perror("write"); exit(1); }
    return fd;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s munmap|remap|dontneed|free|reusable|reusable_noreuse|purgeable|"
                        "file_shared|file_private|malloc|compressed [MB]\n", argv[0]);
        return 2;
    }
    const char *c = argv[1];
    PG = (size_t)getpagesize();
    LEN = (size_t)(argc > 2 ? atol(argv[2]) : 512) << 20;
    NPG = LEN / PG;
    mach_timebase_info(&TB);
    printf("== %s: pid %d, page %zu, %.0f MB, default zone \"%s\"\n", c, getpid(), PG, MB(LEN),
           malloc_get_zone_name(malloc_default_zone()));
    show("baseline");
    char *p;
    uint64_t t0;
    if (!strcmp(c, "munmap") || !strcmp(c, "remap")) {
        p = anon(); touch(p, 0); show("touched");
        t0 = now();
        if (c[0] == 'm') { munmap(p, LEN); printf("  munmap %.2f ms\n", ms(t0, now())); show("after munmap"); hold("after munmap"); p = anon(); }
        else {
            if (mmap(p, LEN, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON | MAP_FIXED, -1, 0) == MAP_FAILED) perror("remap");
            printf("  mmap(MAP_FIXED) %.2f ms\n", ms(t0, now())); show("after MAP_FIXED remap"); hold("after remap");
        }
        retouch(p, "zero-fill refault");
    } else if (!strcmp(c, "dontneed") || !strcmp(c, "free") || !strncmp(c, "reusable", 8)) {
        int adv = c[0] == 'd' ? MADV_DONTNEED : c[0] == 'f' ? MADV_FREE : MADV_FREE_REUSABLE;
        p = anon(); touch(p, 0); show("touched");
        t0 = now();
        if (madvise(p, LEN, adv)) perror("madvise");
        printf("  madvise(%s) %.2f ms\n", c, ms(t0, now()));
        show("after madvise"); hold("after madvise");
        if (!strcmp(c, "reusable")) {
            t0 = now();
            if (madvise(p, LEN, MADV_FREE_REUSE)) perror("MADV_FREE_REUSE");
            printf("  madvise(MADV_FREE_REUSE) %.2f ms\n", ms(t0, now()));
            show("after MADV_FREE_REUSE");
        }
        retouch(p, "re-touch");
    } else if (!strcmp(c, "purgeable")) {
        mach_vm_address_t a = 0;
        kern_return_t kr = mach_vm_allocate(mach_task_self(), &a, LEN, VM_FLAGS_ANYWHERE | VM_FLAGS_PURGABLE);
        if (kr != KERN_SUCCESS) { printf("mach_vm_allocate failed: %d\n", kr); return 1; }
        p = (char *)a; touch(p, 0); show("touched (nonvolatile)");
        int st = VM_PURGABLE_VOLATILE;
        t0 = now();
        kr = mach_vm_purgable_control(mach_task_self(), a, VM_PURGABLE_SET_STATE, &st);
        printf("  set VOLATILE %.3f ms (kr %d)\n", ms(t0, now()), kr);
        show("after VOLATILE"); hold("volatile: apply pressure now to see a purge");
        st = VM_PURGABLE_NONVOLATILE;
        t0 = now();
        kr = mach_vm_purgable_control(mach_task_self(), a, VM_PURGABLE_SET_STATE, &st);
        printf("  set NONVOLATILE %.3f ms (kr %d), previous state: %s\n", ms(t0, now()), kr,
               (st & VM_PURGABLE_STATE_MASK) == VM_PURGABLE_EMPTY ? "EMPTY (purged, contents lost)" : "VOLATILE (kept)");
        show("after NONVOLATILE");
        retouch(p, "re-touch");
    } else if (!strcmp(c, "file_shared") || !strcmp(c, "file_private")) {
        char path[1024];
        int fd = mkfile(path), priv = c[5] == 'p';
        show("file written (page cache)");
        p = mmap(NULL, LEN, priv ? PROT_READ | PROT_WRITE : PROT_READ, priv ? MAP_PRIVATE : MAP_SHARED, fd, 0);
        if (p == MAP_FAILED) { perror("mmap file"); return 1; }
        unsigned sum = 0;
        t0 = now();
        for (size_t i = 0; i < NPG; i++) sum += (unsigned char)((volatile char *)p)[i * PG];
        double t = ms(t0, now());
        printf("  read every page %.2f ms (%.4f ms/MB, sum %u)\n", t, t / MB(LEN), sum);
        show("after reading all pages");
        if (priv) {
            t0 = now();
            for (size_t i = 0; i < NPG; i += 10) p[i * PG] = 1;
            printf("  CoW-wrote 10%% of pages %.2f ms\n", ms(t0, now()));
            show("after writing 10% of pages");
        }
        hold("file mapped");
        munmap(p, LEN); close(fd); unlink(path); show("after munmap+unlink");
    } else if (!strcmp(c, "malloc")) {
        size_t n = LEN >> 20;
        char **b = calloc(n, sizeof *b);
        for (size_t i = 0; i < n; i++) { b[i] = malloc(1 << 20); memset(b[i], 1, 1 << 20); }
        show("1 MB blocks malloc'd+written");
        for (size_t i = 0; i < n; i++) free(b[i]);
        show("immediately after free");
        for (int s = 0; s < 4; s++) { sleep(8); show("idle +8 s"); }
        size_t r = malloc_zone_pressure_relief(NULL, 0);
        printf("  malloc_zone_pressure_relief returned %.1f MB\n", MB(r));
        show("after pressure_relief"); hold("after free");
        t0 = now();
        for (size_t i = 0; i < n; i++) { b[i] = malloc(1 << 20); for (size_t j = 0; j < (1u << 20); j += PG) b[i][j] = 1; }
        double t = ms(t0, now());
        printf("  re-malloc + touch %.2f ms (%.4f ms/MB)\n", t, t / MB(LEN));
        show("after re-malloc");
    } else if (!strcmp(c, "compressed")) {
        int secs = getenv("WAIT") ? atoi(getenv("WAIT")) : 90;
        p = anon(); touch(p, 1); show("filled (half-random pages)");
        printf("  now, in another terminal: memory_pressure -l warn   (Ctrl-C it before %d s)\n", secs);
        fflush(stdout);
        for (int t = 0; t < secs; t += 5) { sleep(5); show("idle"); }
        retouch(p, "re-touch (decompress)");
    } else { fprintf(stderr, "unknown case %s\n", c); return 2; }
    return 0;
}
