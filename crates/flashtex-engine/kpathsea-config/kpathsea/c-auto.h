/* c-auto.h for the vendored kpathsea (third_party/kpathsea), hand-written.

   Upstream generates this file with configure from c-auto.in. FlashTeX builds
   kpathsea with the `cc` crate instead of autotools, so the answers configure
   would give on the two supported hosts -- macOS (arm64/x86_64) and 64-bit
   Linux with glibc -- are written out here. Every macro below appears in
   third_party/kpathsea/c-auto.in; the ones not defined here are left
   undefined there as well.

   Deliberately undefined: MAKE_TEX_*_BY_DEFAULT. The engine never spawns
   mktextfm/mktexfmt/mktexpk; a missing file is reported, not generated. That
   is also what `kpsewhich` does for a plain lookup, which is the reference
   the resolver is measured against. */

#ifndef KPATHSEA_C_AUTO_H
#define KPATHSEA_C_AUTO_H

#define KPSEVERSION "kpathsea version 6.4.2"

#define HAVE_ASSERT_H 1
#define HAVE_DECL_ISASCII 1
#define HAVE_DECL_PUTENV 1
#define HAVE_DIRENT_H 1
#define HAVE_DLFCN_H 1
#define HAVE_FLOAT_H 1
#define HAVE_FSEEKO 1
#define HAVE_GETCWD 1
#define HAVE_INTTYPES_H 1
#define HAVE_LIMITS_H 1
#define HAVE_MEMCMP 1
#define HAVE_MEMCPY 1
#define HAVE_MKSTEMP 1
#define HAVE_MKTEMP 1
#define HAVE_PUTENV 1
#define HAVE_PWD_H 1
#define HAVE_STDINT_H 1
#define HAVE_STDIO_H 1
#define HAVE_STDLIB_H 1
#define HAVE_STRCHR 1
#define HAVE_STRINGS_H 1
#define HAVE_STRING_H 1
#define HAVE_STRRCHR 1
#define HAVE_SYS_PARAM_H 1
#define HAVE_SYS_STAT_H 1
#define HAVE_SYS_TYPES_H 1
#define HAVE_UNISTD_H 1
#define HAVE_WCHAR_H 1
#if defined(__linux__)
#define HAVE_STRUCT_STAT_ST_MTIM 1
#endif

#define SIZEOF_LONG 8
#define STDC_HEADERS 1

#define PACKAGE "kpathsea"
#define PACKAGE_NAME "Kpathsea"
#define PACKAGE_TARNAME "kpathsea"
#define PACKAGE_VERSION "6.4.2"
#define PACKAGE_STRING "Kpathsea 6.4.2"
#define PACKAGE_BUGREPORT "tex-k@tug.org"
#define PACKAGE_URL ""
#define VERSION "6.4.2"

#endif /* !KPATHSEA_C_AUTO_H */
