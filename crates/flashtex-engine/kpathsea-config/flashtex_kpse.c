/* flashtex_kpse.c -- the few kpathsea entry points FlashTeX calls, with a
   plain C ABI (no `boolean`, no enum) so the Rust side needs no bindgen.

   The initialisation sequence is kpsewhich's own (kpsewhich.c, main and
   init_more): kpathsea_new, kpathsea_set_program_name, the `engine`
   variable, mktex disabled for every format, kpathsea_init_prog. Lookups use
   must_exist=false, as a plain `kpsewhich NAME` does. That is what makes the
   results comparable one-for-one with kpsewhich.

   This file is part of flashtex-engine (GPL-2.0-or-later); kpathsea itself is
   LGPL-2.1-or-later (third_party/kpathsea/COPYING.LESSERv2). */

#include <kpathsea/kpathsea.h>
#include <kpathsea/tex-file.h>
#include <string.h>
#include <stdlib.h>

/* ENV is a NULL-terminated list of name, value pairs set in the environment
   before any configuration is read. The bundle resolver uses it; the TeX
   Live one passes none. */
void *flashtex_kpse_new(const char *argv0, const char *progname, const char *engine,
                        const char *const *env)
{
  kpathsea kpse = kpathsea_new();
  kpathsea_set_program_name(kpse, argv0, progname);
  if (engine && *engine)
    kpathsea_xputenv(kpse, "engine", engine);
  for (; env && env[0] && env[1]; env += 2)
    kpathsea_xputenv(kpse, env[0], env[1]);
  kpathsea_set_program_enabled(kpse, kpse_pk_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_mf_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_tex_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_tfm_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_fmt_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_ofm_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_ocp_format, false, kpse_src_cmdline - 1);
  kpathsea_init_prog(kpse, uppercasify(kpse->program_name), 600, NULL, NULL);
  return kpse;
}

/* The kpse_file_format_type whose name (as `kpsewhich --help-formats`
   prints it) is TYPE, or -1. */
int flashtex_kpse_format(void *k, const char *type)
{
  kpathsea kpse = (kpathsea) k;
  int f;
  for (f = 0; f != kpse_last_format; f++) {
    if (!kpse->format_info[f].type)
      kpathsea_init_format(kpse, (kpse_file_format_type) f);
    if (kpse->format_info[f].type && strcmp(kpse->format_info[f].type, type) == 0)
      return f;
  }
  return -1;
}

/* Malloc'd path, or NULL. Free with flashtex_kpse_free. */
char *flashtex_kpse_find(void *k, const char *name, int format)
{
  return kpathsea_find_file((kpathsea) k, name, (kpse_file_format_type) format, false);
}

/* Malloc'd value of a texmf.cnf variable after expansion, or NULL. */
char *flashtex_kpse_var_value(void *k, const char *var)
{
  return kpathsea_var_value((kpathsea) k, var);
}

void flashtex_kpse_putenv(void *k, const char *var, const char *value)
{
  kpathsea_xputenv((kpathsea) k, var, value);
}

void flashtex_kpse_free(void *p)
{
  free(p);
}

void flashtex_kpse_finish(void *k)
{
  kpathsea_finish((kpathsea) k);
}

/* kpathsea_in_name_ok / kpathsea_out_name_ok: may FNAME be read (WRITE=0)
   or written (WRITE=1) under texmf.cnf's openin_any / openout_any? As
   tex.ch calls them, not silent: a refusal is reported on stderr. */
int flashtex_kpse_name_ok(void *k, const char *fname, int write)
{
  kpathsea kpse = (kpathsea) k;
  return write ? kpathsea_out_name_ok(kpse, fname) : kpathsea_in_name_ok(kpse, fname);
}
