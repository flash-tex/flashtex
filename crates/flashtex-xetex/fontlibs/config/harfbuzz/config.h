/* config.h for TeX Live 2026's HarfBuzz (third_party/harfbuzz), FlashTeX.
 *
 * TeX Live's libs/harfbuzz/configure writes config.h from its config.h.in
 * (libs/harfbuzz/configure.ac). This file is that configure's output, run
 * unmodified on macOS 26 / Apple clang (2026-10-04, at texlive-2026.1,
 * commit 6a300188053b8f2ded89dbd52293732a706b9c0e), with two changes:
 *
 *  1. HAVE_GRAPHITE2 is not defined. TeX Live builds HarfBuzz with Graphite2;
 *     FlashTeX adds Graphite2 in phase S2 (third_party/harfbuzz/README.md).
 *  2. The header and function checks configure makes are made here per
 *     target: header checks with __has_include, the POSIX-only functions
 *     only off Windows. On macOS and on glibc Linux the result is what
 *     configure writes there (macOS: every check below passes; glibc 2.26+
 *     has no <xlocale.h>, which __has_include reports).
 *
 * None of these defines changes shaping: they select HarfBuzz's portable
 * fallbacks for memory mapping, locale-independent number parsing and
 * memory allocation (hb-blob.cc, hb-number.cc, hb-common.cc).
 */

/* The normal alignment of 'struct{char;}', in bytes. */
#define ALIGNOF_STRUCT_CHAR__ 1

/* Use native OpenType Layout backend */
#define HAVE_OT 1

/* define if the compiler supports basic C++11 syntax */
#define HAVE_CXX11 1

/* Use Graphite library: phase S2 (see above). */
/* #undef HAVE_GRAPHITE2 */

#define HAVE_ATEXIT 1
#define HAVE_ROUND 1
#define HAVE_STDBOOL_H 1
#define HAVE_STDINT_H 1
#define HAVE_STDIO_H 1
#define HAVE_STDLIB_H 1
#define HAVE_STRING_H 1
#define HAVE_INTTYPES_H 1
#define HAVE_WCHAR_H 1
#define HAVE_SYS_STAT_H 1
#define HAVE_SYS_TYPES_H 1
#define STDC_HEADERS 1

#if defined(__has_include)
# if __has_include(<strings.h>)
#  define HAVE_STRINGS_H 1
# endif
# if __has_include(<unistd.h>)
#  define HAVE_UNISTD_H 1
# endif
# if __has_include(<sys/mman.h>)
#  define HAVE_SYS_MMAN_H 1
# endif
# if __has_include(<xlocale.h>)
#  define HAVE_XLOCALE_H 1
# endif
#endif

#if !defined(_WIN32)
# define HAVE_GETPAGESIZE 1
# define HAVE_ISATTY 1
# define HAVE_MMAP 1
# define HAVE_MPROTECT 1
# define HAVE_NEWLOCALE 1
# define HAVE_POSIX_MEMALIGN 1
# define HAVE_STRTOD_L 1
# define HAVE_SYSCONF 1
#endif

#define PACKAGE_BUGREPORT "tex-k@tug.org"
#define PACKAGE_NAME "harfbuzz (TeX Live)"
#define PACKAGE_STRING "harfbuzz (TeX Live) 12.3.2"
#define PACKAGE_TARNAME "harfbuzz--tex-live-"
#define PACKAGE_URL ""
#define PACKAGE_VERSION "12.3.2"

/* AC_USE_SYSTEM_EXTENSIONS, as configure writes it. */
#ifndef _ALL_SOURCE
# define _ALL_SOURCE 1
#endif
#ifndef _DARWIN_C_SOURCE
# define _DARWIN_C_SOURCE 1
#endif
#ifndef __EXTENSIONS__
# define __EXTENSIONS__ 1
#endif
#ifndef _GNU_SOURCE
# define _GNU_SOURCE 1
#endif
#ifndef _HPUX_ALT_XOPEN_SOCKET_API
# define _HPUX_ALT_XOPEN_SOCKET_API 1
#endif
#ifndef _NETBSD_SOURCE
# define _NETBSD_SOURCE 1
#endif
#ifndef _OPENBSD_SOURCE
# define _OPENBSD_SOURCE 1
#endif
#ifndef _POSIX_PTHREAD_SEMANTICS
# define _POSIX_PTHREAD_SEMANTICS 1
#endif
#ifndef __STDC_WANT_IEC_60559_ATTRIBS_EXT__
# define __STDC_WANT_IEC_60559_ATTRIBS_EXT__ 1
#endif
#ifndef __STDC_WANT_IEC_60559_BFP_EXT__
# define __STDC_WANT_IEC_60559_BFP_EXT__ 1
#endif
#ifndef __STDC_WANT_IEC_60559_DFP_EXT__
# define __STDC_WANT_IEC_60559_DFP_EXT__ 1
#endif
#ifndef __STDC_WANT_IEC_60559_EXT__
# define __STDC_WANT_IEC_60559_EXT__ 1
#endif
#ifndef __STDC_WANT_IEC_60559_FUNCS_EXT__
# define __STDC_WANT_IEC_60559_FUNCS_EXT__ 1
#endif
#ifndef __STDC_WANT_IEC_60559_TYPES_EXT__
# define __STDC_WANT_IEC_60559_TYPES_EXT__ 1
#endif
#ifndef __STDC_WANT_LIB_EXT2__
# define __STDC_WANT_LIB_EXT2__ 1
#endif
#ifndef __STDC_WANT_MATH_SPEC_FUNCS__
# define __STDC_WANT_MATH_SPEC_FUNCS__ 1
#endif
#ifndef _TANDEM_SOURCE
# define _TANDEM_SOURCE 1
#endif
#ifndef _XOPEN_SOURCE
/* # undef _XOPEN_SOURCE */
#endif
