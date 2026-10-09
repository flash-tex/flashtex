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
#include <kpathsea/hash.h>
#include <string.h>
#include <stdlib.h>

/* The mktex discard flag a new instance starts with
   (flashtex_kpse_set_make_tex_discard_errors). */
static int flashtex_make_tex_discard_errors;

/* A list kpathsea_db_search returned, given back whole: its names (each
   allocated for the list), its array and itself. str_list_free frees the
   array only, which lost every name found once per lookup a resident host
   repeats (macOS `leaks`, lane MEMORY-SAFETY, 2026-10-09). */
static void flashtex_free_found(str_list_type *l)
{
  unsigned i;
  for (i = 0; i < STR_LIST_LENGTH(*l); i++)
    free(STR_LIST_ELT(*l, i));
  str_list_free(l);
  free(l);
}

/* elt_in_db (db.c): whether DB_DIR, an ls-R database's directory, is a
   prefix of PATH_ELT. */
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

/* Whether a database covers PATH_ELT, as kpathsea_db_search decides. */
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

/* The directories kpathsea has expanded FORMAT's search path to on disk:
   for each path element that is not `!!' and that no ls-R database covers,
   the element's root (the part before `//', present or not) and then the
   directories kpathsea_element_dirs gives (its cached `//' expansion). A
   change to one of them (a subdirectory made or removed, the root
   appearing) can make that expansion stale: flashtex_kpse_forget_disk_dirs.
   A malloc'd NULL-terminated array of malloc'd paths; free with
   flashtex_kpse_free_list. (#1493, re-review: a resident host never
   searched a TEXMFHOME subdirectory made after it started, nor a
   TEXINPUTS `dir//' whose `dir' was missing then; a fresh process does.) */
char **flashtex_kpse_disk_dirs(void *k, int format)
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
      if (flashtex_covered(kpse, elt))
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
   expands those elements again from the disk, as a fresh process does.
   kpathsea caches an expansion once per process (elt-dirs.c, `cache'), an
   empty one included. Expansions of database-covered elements are kept
   (walking the distribution's trees again costs seconds). */
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
    free(kpse->link_table.tails);
    kpse->link_table.buckets = NULL;
    kpse->link_table.tails = NULL;
    kpse->link_table.size = 0;
  }
}

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

/* The directory of ls-R's index cache (db.c, packed_build), copied; NULL
   for none. Called before an instance reads ls-R. */
void flashtex_kpse_set_lsr_cache(const char *dir)
{
  free(flashtex_lsr_cache_dir);
  flashtex_lsr_cache_dir = dir ? xstrdup(dir) : NULL;
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
      /* found in this element's database (kpathsea's own match of the
         element, and the file on disk): the elements after it were not
         searched */
      if (found) {
        str_list_type *hit = kpathsea_db_search(kpse, xbasename(found), elt, false);
        if (hit) {
          if (STR_LIST_LENGTH(*hit) > 0 && STR_LIST_ELT(*hit, 0)
              && strcmp(STR_LIST_ELT(*hit, 0), found) == 0)
            done = 1;
          flashtex_free_found(hit);
        }
      }
      continue;
    }
    probe = kpathsea_db_search(kpse, "flashtex-lookup-memo-probe", elt, false);
    if (probe) {
      flashtex_free_found(probe);
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

/* Whether an ls-R database lists NAME, or NAME with one of FORMAT's
   suffixes, where no readable file is (kpathsea_db_search passes over such
   an entry, and finds the file once it is there, with no directory of the
   search changing), or has an alias for one of them: then the memo does not
   keep the lookup (resolver.rs, `Memo`). Every entry counts, whatever tree
   it is in (the conservative answer). */
int flashtex_kpse_db_hazard(void *k, int format, const char *name)
{
  kpathsea kpse = (kpathsea) k;
  kpse_format_info_type *f;
  const_string *lists[2];
  const_string *ext;
  int l, hazard = 0;
  if (kpse->db.buckets == NULL)
    return 0;
  if (!kpse->format_info[format].type)
    kpathsea_init_format(kpse, (kpse_file_format_type) format);
  f = &kpse->format_info[format];
  lists[0] = f->suffix;
  lists[1] = f->alt_suffix;
  for (l = -1; l < 2 && !hazard; l++) {
    const_string *exts = l < 0 ? NULL : lists[l];
    if (l >= 0 && !exts)
      continue;
    for (ext = exts; l < 0 || *ext; ext = ext ? ext + 1 : NULL) {
      string n = l < 0 ? xstrdup(name) : concat(name, *ext);
      const_string *dirs = flashtex_db_lookup(kpse, n);
      const_string *d;
      for (d = dirs; d && *d && !hazard; d++) {
        string file = concat(*d, n);
        if (!kpathsea_readable_file(kpse, file))
          hazard = 1;
        free(file);
      }
      free((void *) dirs);
      if (!hazard && kpse->alias_db.buckets) {
        const_string *a = hash_lookup(kpse->alias_db, n);
        hazard = a && *a;
        free((void *) a);
      }
      free(n);
      if (l < 0 || hazard)
        break;
    }
  }
  return hazard;
}

/* The names kpathsea_find_file_generic tries for NAME of FORMAT, in any
   order: NAME (where the format allows a name without its suffix, or NAME
   has one), and NAME with each of the format's suffixes appended (where it
   has none), as tex-file.c's target_asis_name and target_suffixed_names
   build them (fontmap aliases aside: flashtex_kpse_has_alias). A malloc'd
   NULL-terminated list (flashtex_kpse_free_list). The incremental journal
   checks the directories a lookup searched for entries under these names
   that kpathsea passes over (system.rs, `note_lookup`; #1562). */
char **flashtex_kpse_try_names(void *k, int format, const char *name)
{
  kpathsea kpse = (kpathsea) k;
  kpse_format_info_type *f;
  const_string *ext;
  size_t name_len = strlen(name), n = 0, cap = 4;
  int has_potential_suffix = 0;
  char **out = (char **) xmalloc(cap * sizeof(char *));
  if (!kpse->format_info[format].type)
    kpathsea_init_format(kpse, (kpse_file_format_type) format);
  f = &kpse->format_info[format];
  for (ext = f->suffix; ext && !has_potential_suffix && *ext; ext++) {
    size_t l = strlen(*ext);
    has_potential_suffix = name_len >= l && FILESTRCASEEQ(*ext, name + name_len - l);
  }
  for (ext = f->alt_suffix; ext && !has_potential_suffix && *ext; ext++) {
    size_t l = strlen(*ext);
    has_potential_suffix = name_len >= l && FILESTRCASEEQ(*ext, name + name_len - l);
  }
  if (has_potential_suffix || !f->suffix_search_only)
    out[n++] = xstrdup(name);
  for (ext = f->suffix; ext && !has_potential_suffix && *ext; ext++) {
    if (n + 2 > cap) {
      cap *= 2;
      out = (char **) xrealloc(out, cap * sizeof(char *));
    }
    out[n++] = concat(name, *ext);
  }
  out[n] = NULL;
  return out;
}

/* Whether kpathsea_make_tex may run FORMAT's mktex script (tex-make.c:
   the format has a program, and it is enabled). Where it may not, a
   lookup with must_exist that found nothing searched no more than one
   without it: an ls-R tree (`!!`) is not searched on disk either way. */
int flashtex_kpse_make_enabled(void *k, int format)
{
  kpathsea kpse = (kpathsea) k;
  if (!kpse->format_info[format].type)
    kpathsea_init_format(kpse, (kpse_file_format_type) format);
  return kpse->format_info[format].program && kpse->format_info[format].program_enabled_p;
}
