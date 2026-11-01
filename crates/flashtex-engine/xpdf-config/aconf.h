/*
 * aconf.h for TeX Live's xpdf (third_party/xpdf), as TeX Live's
 * libs/xpdf/configure generates it from aconf.h.in on the hosts we build on
 * (macOS and Linux, 64-bit). TeX Live passes none of --enable-a4-paper,
 * --enable-opi or --enable-multithreaded, and USE_EXCEPTIONS stays
 * undefined. Only the header and function checks depend on the host, and
 * every host we build on has them. `configure` renames PACKAGE_* to
 * XPDF_PACKAGE_*.
 *
 * Template copyright 2002-2003 Glyph & Cog, LLC; this instance is part of
 * flashtex-engine (GPL-2.0-or-later).
 */

#ifndef ACONF_H
#define ACONF_H

/*
 * Enable C++ exceptions.
 */
/* #undef USE_EXCEPTIONS */

/* #undef A4_PAPER */
#define HAVE_DIRENT_H 1
/* #undef HAVE_FSEEK64 */
#define HAVE_FSEEKO 1
#define HAVE_INTTYPES_H 1
#define HAVE_MKSTEMP 1
#define HAVE_MKSTEMPS 1
#define HAVE_POPEN 1
#define HAVE_STDINT_H 1
#define HAVE_STDIO_H 1
#define HAVE_STDLIB_H 1
#define HAVE_STRINGS_H 1
#define HAVE_STRING_H 1
#define HAVE_SYS_SELECT_H 1
#define HAVE_SYS_STAT_H 1
#define HAVE_SYS_TYPES_H 1
#define HAVE_TIME_H 1
#define HAVE_UNISTD_H 1
#define HAVE_WCHAR_H 1
/* #undef MULTITHREADED */
/* #undef OPI_SUPPORT */
#define XPDF_PACKAGE_BUGREPORT "tex-k@tug.org"
#define XPDF_PACKAGE_NAME "xpdf (TeX Live)"
#define XPDF_PACKAGE_STRING "xpdf (TeX Live) 4.06"
#define XPDF_PACKAGE_TARNAME "xpdf--tex-live-"
#define XPDF_PACKAGE_URL ""
#define XPDF_PACKAGE_VERSION "4.06"
#define STDC_HEADERS 1

/* AC_USE_SYSTEM_EXTENSIONS (from KPSE_BASIC) */
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
#ifndef _NETBSD_SOURCE
# define _NETBSD_SOURCE 1
#endif
#ifndef _OPENBSD_SOURCE
# define _OPENBSD_SOURCE 1
#endif

#endif
