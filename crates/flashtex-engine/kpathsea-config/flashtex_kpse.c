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
#include <kpathsea/tex-glyph.h>
#include <kpathsea/proginit.h>
#include <kpathsea/pathsearch.h>
#include <kpathsea/db.h>
#include <kpathsea/fontmap.h>
#include <kpathsea/readable.h>
#include <kpathsea/str-list.h>
#include <kpathsea/str-llist.h>
#include <string.h>
#include <stdlib.h>

/* The mktex discard flag a new instance starts with
   (flashtex_kpse_set_make_tex_discard_errors). */
static int flashtex_make_tex_discard_errors;

/* ENV is a NULL-terminated list of name, value pairs set in the environment
   before any configuration is read. The bundle resolver uses it; the TeX
   Live one passes none. */
void *flashtex_kpse_new(const char *argv0, const char *progname, const char *engine,
                        const char *const *env, int mktextfm)
{
  kpathsea kpse = kpathsea_new();
  kpathsea_set_program_name(kpse, argv0, progname);
  if (engine && *engine)
    kpathsea_xputenv(kpse, "engine", engine);
  for (; env && env[0] && env[1]; env += 2)
    kpathsea_xputenv(kpse, env[0], env[1]);
  /* mktexpk: a web2c engine leaves it to the program (pdftex.web enables it
     at the lowest level when PDF output starts, flashtex_kpse_init_pk), so
     that texmf.cnf's MKTEXPK and the environment can turn it off; kpsewhich
     and the bundle keep it off. */
  if (!mktextfm)
    kpathsea_set_program_enabled(kpse, kpse_pk_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_mf_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_tex_format, false, kpse_src_cmdline - 1);
  /* MKTEXTFM: web2c's maininit enables it at the lowest level
     (MAKE_TEX_TFM_BY_DEFAULT), so that texmf.cnf and the environment can
     turn it off; kpsewhich and the bundle keep it off. */
  if (mktextfm)
    kpathsea_set_program_enabled(kpse, kpse_tfm_format, true, kpse_src_compile);
  else
    kpathsea_set_program_enabled(kpse, kpse_tfm_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_fmt_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_ofm_format, false, kpse_src_cmdline - 1);
  kpathsea_set_program_enabled(kpse, kpse_ocp_format, false, kpse_src_cmdline - 1);
  kpathsea_init_prog(kpse, uppercasify(kpse->program_name), 600, NULL, NULL);
  kpse->make_tex_discard_errors = flashtex_make_tex_discard_errors;
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

/* kpathsea_find_file with MUST_EXIST, as web2c's open_input calls it,
   telling whether an mktex script (mktextfm) made the file: the lookup is
   first made without the script, and only if that finds nothing, with it,
   which returns what a single lookup would. Malloc'd path, or NULL. */
char *flashtex_kpse_find_ex(void *k, const char *name, int format, int must_exist,
                            int *made)
{
  kpathsea kpse = (kpathsea) k;
  kpse_format_info_type *f;
  char *r;
  *made = 0;
  if (!kpse->format_info[format].type)
    kpathsea_init_format(kpse, (kpse_file_format_type) format);
  f = &kpse->format_info[format];
  if (!must_exist || !f->program_enabled_p)
    return kpathsea_find_file(kpse, name, (kpse_file_format_type) format, must_exist);
  f->program_enabled_p = false;
  r = kpathsea_find_file(kpse, name, (kpse_file_format_type) format, must_exist);
  f->program_enabled_p = true;
  if (r)
    return r;
  r = kpathsea_find_file(kpse, name, (kpse_file_format_type) format, must_exist);
  *made = r != NULL;
  return r;
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

/* tex.ch [49.1265]: in \batchmode the mktex scripts' errors are discarded
   (kpse_make_tex_discard_errors). The flag is kept here too, and
   flashtex_kpse_new gives it to every instance it starts, so that K may be
   NULL: a resolver whose kpathsea has not started yet (it may start at the
   first lookup) gets the flag when it does. */
void flashtex_kpse_set_make_tex_discard_errors(void *k, int discard)
{
  flashtex_make_tex_discard_errors = discard != 0;
  if (k)
    ((kpathsea) k)->make_tex_discard_errors = flashtex_make_tex_discard_errors;
}

/* The flag of instance K, or (K NULL) the one a new instance starts with. */
int flashtex_kpse_get_make_tex_discard_errors(void *k)
{
  return k ? ((kpathsea) k)->make_tex_discard_errors != 0
           : flashtex_make_tex_discard_errors;
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

/* `kpsewhich -all NAME`: every match, in search order, as a malloc'd
   NULL-terminated array of malloc'd paths (never NULL). Free with
   flashtex_kpse_free_list. fmtutil reads every fmtutil.cnf this way. */
char **flashtex_kpse_find_all(void *k, const char *name, int format)
{
  return kpathsea_find_file_generic((kpathsea) k, name,
                                    (kpse_file_format_type) format, false, true);
}

void flashtex_kpse_free_list(char **list)
{
  char **p;
  if (!list)
    return;
  for (p = list; *p; p++)
    free(*p);
  free(list);
}

/* pdftex.web's `@<Initialize variables for \.{PDF} output@>`:
   kpse_init_prog(PREFIX, DPI, MODE, nil) and
   kpse_set_program_enabled(kpse_pk_format, 1, kpse_src_compile). MODE is
   NULL for no \pdfpkmode. */
void flashtex_kpse_init_pk(void *k, const char *prefix, unsigned dpi, const char *mode)
{
  kpathsea kpse = (kpathsea) k;
  kpathsea_init_prog(kpse, prefix, dpi, mode, NULL);
  kpathsea_set_program_enabled(kpse, kpse_pk_format, 1, kpse_src_compile);
}

/* writet3.c's kpse_find_pk(NAME, DPI, &font_ret): the malloc'd path or NULL;
   with a path, *RET_NAME (malloc'd) and *RET_DPI are font_ret's name and dpi,
   and *MADE says whether mktexpk made the file. Without MAKE, mktexpk is
   not run whatever the settings (the display-list writer's look). */
char *flashtex_kpse_find_pk(void *k, const char *name, unsigned dpi, int make,
                            char **ret_name, unsigned *ret_dpi, int *made)
{
  kpathsea kpse = (kpathsea) k;
  kpse_glyph_file_type g;
  kpse_format_info_type *f;
  boolean enabled;
  char *r;
  g.name = NULL;
  g.dpi = 0;
  g.format = kpse_pk_format;
  g.source = kpse_glyph_source_normal;
  if (!kpse->format_info[kpse_pk_format].type)
    kpathsea_init_format(kpse, kpse_pk_format);
  f = &kpse->format_info[kpse_pk_format];
  enabled = f->program_enabled_p;
  if (!make)
    f->program_enabled_p = false;
  r = kpathsea_find_glyph(kpse, name, dpi, kpse_pk_format, &g);
  f->program_enabled_p = enabled;
  *ret_name = NULL;
  *ret_dpi = 0;
  *made = 0;
  if (r) {
    *ret_name = xstrdup(g.name ? g.name : "");
    *ret_dpi = g.dpi;
    *made = g.source == kpse_glyph_source_maketex;
  }
  return r;
}

/* ---- the session's lookup memo (resolver.rs, `Memo`) ----------------- */

/* The search path of FORMAT (kpse_format_info_type.path), as
   kpathsea_find_file_generic searches it; not to be freed. NULL if the
   format has none. */
const char *flashtex_kpse_format_path(void *k, int format)
{
  kpathsea kpse = (kpathsea) k;
  if (!kpse->format_info[format].type)
    kpathsea_init_format(kpse, (kpse_file_format_type) format);
  return kpse->format_info[format].path;
}

/* Whether NAME has an alias in kpathsea's fontmaps (texfonts.map), which
   kpathsea_find_file_generic looks for as well (for font formats). */
int flashtex_kpse_has_alias(void *k, const char *name)
{
  const_string *a = kpathsea_fontmap_lookup((kpathsea) k, name);
  return a && *a;
}

/* kpathsea_readable_file: a regular file that may be read. */
int flashtex_kpse_readable(void *k, const char *path)
{
  string p = xstrdup(path);
  int r = kpathsea_readable_file((kpathsea) k, p) != NULL;
  free(p);
  return r;
}

/* The directories whose listings decide a lookup of FORMAT that found FOUND
   (NULL: found nothing), in the order kpathsea_path_search_list_generic
   searches them: every directory of every path element it searches on disk
   (a `//` subtree as kpathsea expanded it) up to the one that holds FOUND,
   or (found nothing) all of them. An element searched in an ls-R database
   only (`!!`) depends on the database, which kpathsea reads once per
   process, as it does for every lookup. A malloc'd NULL-terminated list
   (flashtex_kpse_free_list), or NULL where the dependencies are not known:
   an element that is not `!!` but has a database (kpathsea searches its
   disk only sometimes), or FOUND under no element reached. */
char **flashtex_kpse_search_dirs(void *k, int format, const char *found)
{
  kpathsea kpse = (kpathsea) k;
  const char *path = flashtex_kpse_format_path(k, format);
  size_t n = 0, cap = 16, found_dir_len = 0;
  char **out;
  string elt;
  int done = 0, known = 1;
  if (!path)
    return NULL;
  if (found) {
    const char *slash = strrchr(found, '/');
    if (!slash)
      return NULL;
    found_dir_len = (size_t) (slash - found) + 1; /* with the slash */
  }
  out = (char **) xmalloc(cap * sizeof(char *));
  out[0] = NULL;
  for (elt = kpathsea_path_element(kpse, path); elt; elt = kpathsea_path_element(kpse, NULL)) {
    int db_only = elt[0] == '!' && elt[1] == '!';
    str_list_type *probe;
    str_llist_type *dirs;
    str_llist_elt_type *d;
    if (done || !known)
      continue; /* (finish kpathsea's one path iteration) */
    if (db_only)
      elt += 2;
    kpathsea_normalize_path(kpse, elt);
    if (db_only) {
      /* found in this element's database: the elements after it were not
         searched */
      if (found) {
        size_t l = strlen(elt);
        while (l > 0 && elt[l - 1] == '/')
          l--;
        if (l > 0 && strncmp(found, elt, l) == 0 && found[l] == '/')
          done = 1;
      }
      continue;
    }
    probe = kpathsea_db_search(kpse, "flashtex-lookup-memo-probe", elt, false);
    if (probe) {
      str_list_free(probe);
      free(probe);
      known = 0;
      continue;
    }
    dirs = kpathsea_element_dirs(kpse, elt);
    for (d = dirs ? *dirs : NULL; d; d = STR_LLIST_NEXT(*d)) {
      const char *dir = STR_LLIST(*d);
      if (n + 2 > cap) {
        cap *= 2;
        out = (char **) xrealloc(out, cap * sizeof(char *));
      }
      out[n++] = xstrdup(dir);
      out[n] = NULL;
      if (found && strlen(dir) == found_dir_len && strncmp(found, dir, found_dir_len) == 0)
        done = 1;
    }
  }
  if (!known || (found && !done)) {
    flashtex_kpse_free_list(out);
    return NULL;
  }
  return out;
}
