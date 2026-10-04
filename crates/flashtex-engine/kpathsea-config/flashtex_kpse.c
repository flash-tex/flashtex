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
#include <string.h>
#include <stdlib.h>

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

/* db.c's `elt_in_db` (static there): is PATH_ELT under DB_DIR? */
static int flashtex_elt_in_db(const char *db_dir, const char *path_elt)
{
  int found = 0;
  if (db_dir == NULL || *db_dir == 0 || path_elt == NULL || *path_elt == 0)
    return 0;
  while (!found && FILECHARCASEEQ(*db_dir++, *path_elt++)) {
    if (*db_dir == 0)
      found = 1;
    else if (*path_elt == 0)
      break;
  }
  return found;
}

/* Whether a database covers PATH_ELT, as kpathsea_db_search_list decides. */
static int flashtex_covered(kpathsea kpse, const char *elt)
{
  unsigned i;
  if (!kpse->followup_search || kpse->db.buckets == NULL)
    return 0;
  for (i = 0; i < STR_LIST_LENGTH(kpse->db_dir_list); i++)
    if (flashtex_elt_in_db(STR_LIST_ELT(kpse->db_dir_list, i), elt))
      return 1;
  return 0;
}

static void flashtex_push(char ***out, unsigned *n, unsigned *cap, const char *s)
{
  if (*n + 1 >= *cap) {
    *cap *= 2;
    *out = (char **) xrealloc(*out, *cap * sizeof(char *));
  }
  (*out)[(*n)++] = xstrdup(s);
}

/* Every directory a lookup of FORMAT reads on disk rather than through an
   ls-R database, in search order: for each path element that is not `!!'
   and that no database covers (the test kpathsea_db_search_list makes),
   first the element's root (the part before `//', present or not: its
   appearing, or a new subdirectory, changes the expansion), then the
   directories kpathsea_element_dirs expands it to. With WITH_COVERED, also
   those of the elements a database covers but `!!' does not mark: a lookup
   with must_exist searches them on disk when the database has no entry
   (pathsearch.c; TEXMFHOME, TEXMFVAR or TEXMFCONFIG with an ls-R).
   A file or directory added to or removed from one of these can change what
   a lookup finds; nothing else can, since kpathsea reads the databases once.
   A malloc'd NULL-terminated array of malloc'd paths ending in `/'; free
   with flashtex_kpse_free_list. */
char **flashtex_kpse_disk_dirs(void *k, int format, int with_covered)
{
  kpathsea kpse = (kpathsea) k;
  kpse_format_info_type *f;
  string elt;
  unsigned n = 0, cap = 16;
  char **out = (char **) xmalloc(cap * sizeof(char *));
  if (!kpse->format_info[format].type)
    kpathsea_init_format(kpse, (kpse_file_format_type) format);
  f = &kpse->format_info[format];
  if (f->path) {
    for (elt = kpathsea_path_element(kpse, f->path); elt;
         elt = kpathsea_path_element(kpse, NULL)) {
      str_llist_type *dirs;
      str_llist_elt_type *e;
      char *root, *dd;
      if (elt[0] == '!' && elt[1] == '!')
        continue;
      kpathsea_normalize_path(kpse, elt);
      if (flashtex_covered(kpse, elt) && !with_covered)
        continue;
      /* The root: ELT up to `//', with one trailing slash. */
      root = xstrdup(elt);
      dd = strstr(root, "//");
      if (dd)
        dd[1] = 0;
      else if (*root && root[strlen(root) - 1] != '/') {
        char *r2 = concat(root, "/");
        free(root);
        root = r2;
      }
      if (*root)
        flashtex_push(&out, &n, &cap, root);
      free(root);
      dirs = kpathsea_element_dirs(kpse, elt);
      for (e = dirs ? *dirs : NULL; e; e = STR_LLIST_NEXT(*e))
        flashtex_push(&out, &n, &cap, STR_LLIST(*e));
    }
  }
  out[n] = NULL;
  return out;
}

/* Forget the `//' expansions kpathsea cached for path elements no database
   covers, and the directory link counts it cached (which decide whether a
   directory is searched for subdirectories), so that the next search
   expands those elements again from the disk. kpathsea caches an expansion
   once per process (elt-dirs.c, `cache'), an empty one included, so a
   subdirectory made later, or TEXMFHOME made after the start, was never
   searched by a resident host, while a fresh process found it. Called when
   one of flashtex_kpse_disk_dirs' directories changed. Expansions of
   database-covered elements (the distribution's trees) are kept: they are
   searched on disk only by must_exist lookups that miss, and walking them
   again costs seconds. */
void flashtex_kpse_forget_disk_dirs(void *k)
{
  kpathsea kpse = (kpathsea) k;
  unsigned i, j = 0;
  for (i = 0; i < kpse->cache_length; i++) {
    cache_entry c = kpse->the_cache[i];
    str_llist_elt_type *e, *next;
    if (flashtex_covered(kpse, c.key)) {
      kpse->the_cache[j++] = c;
      continue;
    }
    for (e = c.value ? *c.value : NULL; e; e = next) {
      next = STR_LLIST_NEXT(*e);
      free(STR_LLIST(*e));
      free(e);
    }
    free(c.value);
    free((char *) c.key);
  }
  kpse->cache_length = j;
  if (kpse->link_table.size) {
    for (i = 0; i < kpse->link_table.size; i++) {
      hash_element_type *h = kpse->link_table.buckets[i], *hn;
      for (; h; h = hn) {
        hn = h->next;
        free((char *) h->key);
        free(h);
      }
    }
    free(kpse->link_table.buckets);
    kpse->link_table.buckets = NULL;
    kpse->link_table.size = 0;
  }
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
