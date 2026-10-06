/* WASI spike shim (README.md §3): no user database. */
#ifndef WASI_SHIM_PWD_H
#define WASI_SHIM_PWD_H
#include <sys/types.h>
struct passwd { char *pw_name; char *pw_dir; uid_t pw_uid; gid_t pw_gid; };
static inline struct passwd *getpwuid(uid_t u) { (void)u; return 0; }
static inline struct passwd *getpwnam(const char *n) { (void)n; return 0; }
static inline uid_t getuid(void) { return 0; }
#endif
