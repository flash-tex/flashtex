/* db.c: an external database to avoid filesystem lookups.

   Copyright 1994, 1995, 1996, 1997, 2008, 2009, 2011, 2012, 2014, 2016,
   2017 Karl Berry.
   Copyright 1997-2005 Olaf Weber.

   This library is free software; you can redistribute it and/or
   modify it under the terms of the GNU Lesser General Public
   License as published by the Free Software Foundation; either
   version 2.1 of the License, or (at your option) any later version.

   This library is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
   Lesser General Public License for more details.

   You should have received a copy of the GNU Lesser General Public License
   along with this library; if not, see <http://www.gnu.org/licenses/>.  */

#include <kpathsea/config.h>
#include <kpathsea/absolute.h>
#include <kpathsea/c-stat.h>
#include <kpathsea/c-fopen.h>
#include <kpathsea/c-pathch.h>
#include <kpathsea/db.h>
#include <kpathsea/hash.h>
#include <kpathsea/line.h>
#include <kpathsea/pathsearch.h>
#include <kpathsea/readable.h>
#include <kpathsea/str-list.h>
#include <kpathsea/tex-file.h>
#include <kpathsea/variable.h>

#ifndef DB_HASH_SIZE
/* Based on the size of 2014 texmf-dist/ls-R, about 130,000 entries.  */
#define DB_HASH_SIZE 64007
#endif
#ifndef DB_NAME
#define DB_NAME "ls-R"
#endif
#ifndef DB_NAME_LC
#define DB_NAME_LC "ls-r"
#endif

/* In the loop in init() below where we check for DB_NAME_LC and DB_NAME
   being the same file, it's convenient to ignore the first of a pair.
   So put the canonical name second.  Sure wish I hadn't used a capital
   letter in the name in the first place.  */
/* We use non-const strings initialized with string constants in order
   to avoid some compiler warnings.  */
static char db_name_lc[] = DB_NAME_LC;
static char db_name[] = DB_NAME;
static string db_names[] = {
    db_name_lc,
    db_name,
    NULL
};

#ifndef ALIAS_NAME
#define ALIAS_NAME "aliases"
#endif
#ifndef ALIAS_HASH_SIZE
#define ALIAS_HASH_SIZE 1009
#endif


/* If DIRNAME contains any element beginning with a `.' (that is more
   than just `./'), return true.  This is to allow ``hidden''
   directories -- ones that don't get searched.  */

static boolean
ignore_dir_p (const_string dirname)
{
  const_string dot_pos = dirname;

  while ((dot_pos = strchr (dot_pos + 1, '.'))) {
    /* If / before and no / after, skip it. */
    if (IS_DIR_SEP_CH (dot_pos[-1]) && dot_pos[1] && !IS_DIR_SEP_CH (dot_pos[1]))
      return true;
  }

  return false;
}

/* FlashTeX change (2026-10-09): ls-R's entries are packed. Each file of
   ls-R used to be one hash_element (key, value and next pointers: 24 bytes,
   32 with the allocator's rounding, one malloc each) in a table of 64,007
   buckets plus their tails: about 9 MB for TeX Live's 248,000 files, in
   every process. Here an entry is 12 bytes, three 32-bit numbers: the
   file name's offset in its ls-R buffer, its directory, and the next entry
   of its bucket. Entries live in fixed chunks (no reallocation), buckets
   are 2^16 head and tail indexes. A bucket's chain is in insertion order,
   as hash.c's is, so a lookup returns the same directories in the same
   order. Files inserted while running (kpathsea_db_insert) still go to
   `kpse->db', now a small hash table, and are returned after the packed
   ones, as they were inserted after them. Only where file names compare
   exactly (no MONOCASE_FILENAMES); elsewhere ls-R goes to `kpse->db' as
   before.  */
#if !defined (MONOCASE_FILENAMES)
#define PACKED_DB 1
#endif

#ifdef PACKED_DB
#define PACKED_NONE 0xffffffffu
#define PACKED_CHUNK_BITS 16
#define PACKED_CHUNK (1u << PACKED_CHUNK_BITS)
#define PACKED_BUCKETS (1u << 16)

typedef struct {
  unsigned key;   /* offset of the name in its directory's buffer */
  unsigned dir;   /* index in `dirs' */
  unsigned next;  /* next entry of the bucket, or PACKED_NONE */
} packed_entry;

typedef struct {
  const_string name;  /* with its trailing slash, as cur_dir was */
  const_string buf;   /* the ls-R buffer its file names are in */
} packed_dir;

struct flashtex_packed_db {
  packed_entry **chunks;
  unsigned nchunks, n;
  packed_dir *dirs;
  unsigned ndirs, dir_cap;
  unsigned *head, *tail;
};

static unsigned
packed_hash (const_string key)
{
  unsigned h = 2166136261u;  /* FNV-1a */
  while (*key)
    h = (h ^ (unsigned char) *key++) * 16777619u;
  return h & (PACKED_BUCKETS - 1);
}

static struct flashtex_packed_db *
packed_db (kpathsea kpse)
{
  struct flashtex_packed_db *p = kpse->flashtex_packed_db;
  if (!p) {
    unsigned b;
    p = (struct flashtex_packed_db *) xcalloc (1, sizeof (*p));
    p->head = (unsigned *) xmalloc (PACKED_BUCKETS * sizeof (unsigned));
    p->tail = (unsigned *) xmalloc (PACKED_BUCKETS * sizeof (unsigned));
    for (b = 0; b < PACKED_BUCKETS; b++)
      p->head[b] = p->tail[b] = PACKED_NONE;
    kpse->flashtex_packed_db = p;
  }
  return p;
}

static packed_entry *
packed_at (struct flashtex_packed_db *p, unsigned i)
{
  return &p->chunks[i >> PACKED_CHUNK_BITS][i & (PACKED_CHUNK - 1)];
}

/* A new directory of buffer BUF; its index.  */
static unsigned
packed_add_dir (struct flashtex_packed_db *p, const_string name,
                const_string buf)
{
  if (p->ndirs == p->dir_cap) {
    p->dir_cap = p->dir_cap ? 2 * p->dir_cap : 1024;
    p->dirs = (packed_dir *) xrealloc (p->dirs, p->dir_cap * sizeof (packed_dir));
  }
  p->dirs[p->ndirs].name = name;
  p->dirs[p->ndirs].buf = buf;
  return p->ndirs++;
}

/* File KEY (in directory DIR's buffer) at the end of its bucket.  */
static void
packed_add (struct flashtex_packed_db *p, const_string key, unsigned dir)
{
  unsigned i = p->n, b = packed_hash (key);
  packed_entry *e;
  if ((i >> PACKED_CHUNK_BITS) == p->nchunks) {
    p->chunks = (packed_entry **) xrealloc (p->chunks,
                    (p->nchunks + 1) * sizeof (packed_entry *));
    p->chunks[p->nchunks++] =
      (packed_entry *) xmalloc (PACKED_CHUNK * sizeof (packed_entry));
  }
  e = packed_at (p, i);
  e->key = (unsigned) (key - p->dirs[dir].buf);
  e->dir = dir;
  e->next = PACKED_NONE;
  if (p->tail[b] == PACKED_NONE)
    p->head[b] = i;
  else
    packed_at (p, p->tail[b])->next = i;
  p->tail[b] = i;
  p->n++;
}
#endif /* PACKED_DB */

/* FlashTeX change (2026-10-09): `hash_lookup (kpse->db, KEY)', with ls-R's
   packed entries first (see above): the directories of every file named
   KEY, in insertion order, null-terminated; NULL if none.  */
const_string *
flashtex_db_lookup (kpathsea kpse, const_string key)
{
#ifdef PACKED_DB
  struct flashtex_packed_db *p = kpse->flashtex_packed_db;
  cstr_list_type ret;
  const_string *more, *r;
  unsigned i;
  if (!p)
    return hash_lookup (kpse->db, key);
  ret = cstr_list_init ();
  for (i = p->head[packed_hash (key)]; i != PACKED_NONE; ) {
    packed_entry *e = packed_at (p, i);
    if (STREQ (key, p->dirs[e->dir].buf + e->key))
      cstr_list_add (&ret, p->dirs[e->dir].name);
    i = e->next;
  }
  more = hash_lookup (kpse->db, key);
  if (more) {
    for (r = more; *r; r++)
      cstr_list_add (&ret, *r);
    free ((void *) more);
  }
  if (STR_LIST (ret))
    cstr_list_add (&ret, NULL);
  return STR_LIST (ret);
#else
  return hash_lookup (kpse->db, key);
#endif
}

/* If no DB_FILENAME, return false (maybe they aren't using this feature).
   Otherwise, add entries from DB_FILENAME to TABLE, and return true.  */

static boolean
db_build (kpathsea kpse, hash_table_type *table,  const_string db_filename)
{
  string line;
  unsigned dir_count = 0, file_count = 0, ignore_dir_count = 0;
  unsigned len = strlen (db_filename) - sizeof (DB_NAME) + 1; /* Keep the /. */
  string top_dir = (string)xmalloc (len + 1);
  string cur_dir = NULL; /* First thing in ls-R might be a filename.  */
  FILE *db_file = fopen (db_filename, FOPEN_R_MODE);
#if defined(MONOCASE_FILENAMES)
  string pp;
#endif /* MONOCASE_FILENAMES */

  strncpy (top_dir, db_filename, len);
  top_dir[len] = 0;

  if (db_file) {
    /* FlashTeX change (2026-10-06): ls-R is read whole, once, and split
       into lines in place, instead of a `read_line' (one character at a
       time, one allocation per line) and an `xstrdup' of each file name.
       The file names stay in the buffer, which is never freed once an
       entry points into it, as the duplicates were never freed. The lines
       are exactly `read_line''s: a line ends at LF, CR or CR LF, the last
       one need not end, and null bytes are dropped.  */
    size_t buf_size = 0, buf_cap = 1 << 20, got;
    string buf, next, buf_end;
#ifdef PACKED_DB
    struct flashtex_packed_db *pdb = packed_db (kpse);
    unsigned cur_dir_index = 0;
#endif
    /* FlashTeX change (2026-10-09): the buffer is the file's size (plus
       one), so that it is read without growing: the blocks a growing
       buffer frees (1, 2 and 4 MB) stay in the process on macOS.  */
    {
      struct stat st;
      if (fstat (fileno (db_file), &st) == 0 && st.st_size > 0)
        buf_cap = (size_t) st.st_size + 1;
    }
    buf = (string) xmalloc (buf_cap + 1);
    while ((got = fread (buf + buf_size, 1, buf_cap - buf_size, db_file)) > 0) {
      buf_size += got;
      if (buf_size == buf_cap) {
        buf_cap *= 2;
        buf = (string) xrealloc (buf, buf_cap + 1);
      }
    }
    buf[buf_size] = 0;
    buf_end = buf + buf_size;
    for (next = buf; next < buf_end; ) {
      string q, w;
      line = next;
      for (q = w = line; q < buf_end && *q != '\n' && *q != '\r'; q++)
        if (*q != 0)
          *w++ = *q;
      if (q < buf_end && *q == '\r' && q + 1 < buf_end && q[1] == '\n')
        next = q + 2;
      else
        next = q < buf_end ? q + 1 : buf_end;
      *w = 0;
      len = w - line;

#if defined(MONOCASE_FILENAMES)
      for (pp = line; *pp; pp++) {
#if defined(_WIN32)
        if (kpathsea_IS_KANJI(kpse, pp))
          pp++;
        else
#endif /* _WIN32 */
          *pp = TRANSFORM(*pp);
      }
#endif /* MONOCASE_FILENAMES */

      /* A line like `/foo:' = new dir foo.  Allow both absolute (/...)
         and explicitly relative (./...) names here.  It's a kludge to
         pass in the directory name with the trailing : still attached,
         but it doesn't actually hurt.  */
      if (len > 0 && line[len - 1] == ':'
          && kpathsea_absolute_p (kpse, line, true)) {
        /* New directory line.  */
        if (!ignore_dir_p (line)) {
          /* If they gave a relative name, prepend full directory name now.  */
          line[len - 1] = DIR_SEP;
          /* Skip over leading `./', it confuses `match' and is just a
             waste of space, anyway.  This will lose on `../', but `match'
             won't work there, either, so it doesn't matter.  */
          cur_dir = *line == '.' ? concat (top_dir, line + 2) : xstrdup (line);
#ifdef PACKED_DB
          cur_dir_index = packed_add_dir (pdb, cur_dir, buf);
#endif
          dir_count++;
        } else {
          cur_dir = NULL;
          ignore_dir_count++;
        }

      /* Ignore blank, `.' and `..' lines.  */
      } else if (*line != 0 && cur_dir   /* a file line? */
                 && !(*line == '.'
                      && (line[1] == 0 || (line[1] == '.' && line[2] == 0))))
      {
        /* Make a new hash table entry with a key of `line' and a data
           of `cur_dir'.  An already-existing identical key is ok, since
           a file named `foo' can be in more than one directory.  Share
           `cur_dir' among all its files (and hence never free it).

           Note that we assume that all names in the ls-R file have already
           been case-smashed to lowercase where appropriate.
        */
#ifdef PACKED_DB
        (void) table;
        packed_add (pdb, line, cur_dir_index);
#else
        hash_insert_normalized (table, line, cur_dir);
#endif
        file_count++;

      } /* else ignore blank lines or top-level files
           or files in ignored directories*/
    }

    if (file_count == 0)
      free (buf);
    xfclose (db_file, db_filename);

    if (file_count == 0) {
      WARNING1 ("kpathsea: %s: No usable entries in ls-R", db_filename);
      WARNING ("kpathsea: See the manual for how to generate ls-R");
      db_file = NULL;
    } else {
      str_list_add (&(kpse->db_dir_list), xstrdup (top_dir));
    }

#ifdef KPSE_DEBUG
    if (KPATHSEA_DEBUG_P (KPSE_DEBUG_HASH)) {
      /* Don't make this a debugging bit, since the output is so
         voluminous, and being able to specify -1 is too useful.
         Instead, let people who want it run the program under
         a debugger and change the variable that way.  */
      boolean hash_summary_only = true;

      DEBUGF4 ("%s: %u entries in %d directories (%d hidden).\n",
               db_filename, file_count, dir_count, ignore_dir_count);
      DEBUGF ("ls-R hash table:");
      hash_print (*table, hash_summary_only);
      fflush (stderr);
    }
#endif /* KPSE_DEBUG */
  }

  free (top_dir);

  return db_file != NULL;
}


/* Insert FNAME into the hash table.  This is for files that get built
   during a run.  We wouldn't want to reread all of ls-R, even if it got
   rebuilt.  */

void
kpathsea_db_insert (kpathsea kpse, const_string passed_fname)
{
  /* We might not have found ls-R, or even had occasion to look for it
     yet, so do nothing if we have no hash table.  */
  if (kpse->db.buckets) {
    const_string dir_part;
    string fname = xstrdup (passed_fname);
    string baseptr = fname + (xbasename (fname) - fname);
    const_string file_part = xstrdup (baseptr);

    *baseptr = '\0';  /* Chop off the filename.  */
    dir_part = fname; /* That leaves the dir, with the trailing /.  */

    /* Note that we do not assuse that these names have been normalized. */
    hash_insert (&(kpse->db), file_part, dir_part);
  }
}

/* Return true if FILENAME could be in PATH_ELT, i.e., if the directory
   part of FILENAME matches PATH_ELT.  Have to consider // wildcards, but
   $ and ~ expansion have already been done.  */

static boolean
match (const_string filename,  const_string path_elt)
{
  const_string original_filename = filename;
  boolean matched = false;

  for (; *filename && *path_elt; filename++, path_elt++) {
    if (FILECHARCASEEQ (*filename, *path_elt)) /* normal character match */
      ;

    else if (IS_DIR_SEP_CH (*path_elt)  /* at // */
             && original_filename < filename && IS_DIR_SEP_CH (path_elt[-1])) {
      while (IS_DIR_SEP_CH (*path_elt))
        path_elt++; /* get past second and any subsequent /'s */
      if (*path_elt == 0) {
        /* Trailing //, matches anything. We could make this part of the
           other case, but it seems pointless to do the extra work.  */
        matched = true;
        break;
      } else {
        /* Intermediate //, have to match rest of PATH_ELT.  */
        for (; !matched && *filename; filename++) {
          /* Try matching at each possible character.  */
          if (IS_DIR_SEP_CH (filename[-1])
              && FILECHARCASEEQ (*filename, *path_elt))
            matched = match (filename, path_elt);
        }
        /* Prevent filename++ when *filename='\0'. */
        break;
      }
    }

    else /* normal character nonmatch, quit */
      break;
  }

  /* If we've reached the end of PATH_ELT, check that we're at the last
     component of FILENAME (that is, no directory separators remaining);
     only then have we matched.  */
  if (!matched && *path_elt == 0) {
    /* Typically PATH_ELT ends with, say, `vf', and FILENAME ends with
       `vf/ptmr.vf'.  In that case, we'll be at the /.  On the other
       hand, if PATH_ELT ended with a / (as in `vf/'), FILENAME being
       the same `vf/ptmr.vf', we'll be at the `p'.
       Upshot: if we're at a dir sep in FILENAME, skip it.  */
    if (IS_DIR_SEP_CH (*filename))
      filename++;

    /* Here are the basic possibilities for the check on being at the
       last component:
       1) PATH_ELT is empty and FILENAME is `ptmr.vf'     => match.
          (we now have original_filename == filename)
       2) PATH_ELT is empty and FILENAME is `foo/ptmr.vf' => no match.
          (we now have original_filename == filename)
       3) PATH_ELT is `vf/' and FILENAME is `vf/ptmr.vf'
          (we are now after the / in each)                 => match.
       4) PATH_ELT is `vf' and FILENAME is `vfoo.ext'
          (we are now after the f in each)                 => no match.
       
       When (the original) PATH_ELT was the empty string, we want to match
       a FILENAME without dir seps.  (This could be argued, and may never
       happen in practice, but is the historical behavior.)  */
    /* if original_filename != filename then original_filename < filename */
    if (original_filename == filename || IS_DIR_SEP_CH (filename[-1])) {
      while (*filename && !IS_DIR_SEP_CH (*filename))
        filename++;
      matched = *filename == 0;
    }
  }

  return matched;
}


/* If DB_DIR is a prefix of PATH_ELT, return true; otherwise false.
   That is, the question is whether to try the db for a file looked up
   in PATH_ELT.  If PATH_ELT == ".", for example, the answer is no. If
   PATH_ELT == "/usr/local/lib/texmf/fonts//tfm", the answer is yes.
   If either string is NULL or empty, return false.

   In practice, ls-R is only needed for lengthy subdirectory
   comparisons, but there's no gain to checking PATH_ELT to see if it is
   a subdir match, since the only way to do that is to do a string
   search in it, which is all we do anyway.  */

static boolean
elt_in_db (const_string db_dir,  const_string path_elt)
{
  boolean found = false;

  /* If both strings are empty or null return false on the grounds that
     it's useless to do anything further with such a strange case (which
     likely never happens).  In theory one could argue that the empty
     string is a prefix of any other string, but let's just declare the
     result otherwise.  */
  if (db_dir == NULL || *db_dir == 0
      || path_elt == NULL || *path_elt == 0)
    return false;
     
  while (!found && FILECHARCASEEQ (*db_dir++, *path_elt++)) {
    /* If we've matched the entire db directory, it's good.  */
    if (*db_dir == 0)
      found = true;

    /* If we've reached the end of PATH_ELT, but not the end of the db
       directory, it's no good.  */
    else if (*path_elt == 0)
      break;
  }

  return found;
}

/* If ALIAS_FILENAME exists, read it into TABLE.  */

static boolean
alias_build (kpathsea kpse, hash_table_type *table,
             const_string alias_filename)
{
  string line, real, alias;
  unsigned count = 0;
  FILE *alias_file = fopen (alias_filename, FOPEN_R_MODE);

  if (alias_file) {
    while ((line = read_line (alias_file)) != NULL) {
      /* comments or empty */
      if (*line == 0 || *line == '%' || *line == '#') {
        ;
      } else {
        /* Each line should have two fields: realname aliasname.  */
        real = line;
        while (*real && ISSPACE (*real))
          real++;
        alias = real;
        while (*alias && !ISSPACE (*alias))
          alias++;
        *alias++ = 0;
        while (*alias && ISSPACE (*alias))
          alias++;
        /* Is the check for errors strong enough?  Should we warn the user
           for potential errors?  */
        if (strlen (real) != 0 && strlen (alias) != 0) {
          /* Stuff in the alias file should be normalized. */
          hash_insert_normalized (table, xstrdup (alias), xstrdup (real));
          count++;
        }
      }
      free (line);
    }

#ifdef KPSE_DEBUG
    if (KPATHSEA_DEBUG_P (KPSE_DEBUG_HASH)) {
      /* As with ls-R above ... */
      boolean hash_summary_only = true;
      DEBUGF2 ("%s: %u aliases.\n", alias_filename, count);
      DEBUGF ("alias hash table:");
      hash_print (*table, hash_summary_only);
      fflush (stderr);
    }
#endif /* KPSE_DEBUG */

    xfclose (alias_file, alias_filename);
  }

  return alias_file != NULL;
}

/* Initialize the path for ls-R files, and read them all into the hash
   table `db'.  If no usable ls-R's found, set kpse->db.buckets to NULL.  */

void
kpathsea_init_db (kpathsea kpse)
{
  const_string db_path;
  string *db_files;
  string *orig_db_files;
  str_list_type unique_list;
  int dbi;
  boolean ok = false;

  assert (sizeof(DB_NAME) == sizeof(DB_NAME_LC));

  db_path = kpathsea_init_format (kpse, kpse_db_format);
  db_files = kpathsea_path_search_list_generic (kpse, db_path, db_names,
                                                true, true);
  orig_db_files = db_files;
  
  /* Mac OS X and others can use a case-insensitive, case-preserving
     filesystem by default, in which case ls-R and ls-r point to the
     same file.  Also, Windows is case-insensitive.  In these cases,
     we want to avoid reading the same file multiple times. */
  dbi = 0;
  unique_list = str_list_init ();
  
  while (db_files[dbi] != NULL) {
    string path1 = db_files[dbi];
    string path2 = db_files[dbi + 1];

    /* first-pass check in case path1/path2 aren't even
       potentially equal; mainly in case the order from
       kpathsea_path_search_list_generic changes. */
    if (path2
        && strcasecmp (path1, path2) == 0
        && same_file_p (path1, path2)) {
      /* they are the same, skip over path1, we'll add path2
         on the next iteration (when it's path1). */
#ifdef KPSE_DEBUG
      if (KPATHSEA_DEBUG_P (KPSE_DEBUG_HASH)) {
        DEBUGF2 ("db:init(): skipping db same_file_p %s, will add %s.\n",
                 path1, path2);
      }
#endif
      free (path1);

    } else {
     /* they are not the same, add path1.  */
#ifdef KPSE_DEBUG
      if (KPATHSEA_DEBUG_P (KPSE_DEBUG_HASH)) {
        DEBUGF1 ("db:init(): using db file %s.\n", path1);
      }
#endif
      str_list_add (&unique_list, path1);
    }

    /* could be more clever and increment by two, but then would
       have to avoid jumping off the end of db_files */
    dbi++;
  }
  
  /* always add a NULL terminator.  */
  str_list_add (&unique_list, NULL);
  
  free (orig_db_files);
  db_files = STR_LIST (unique_list);
  orig_db_files = db_files;

  /* Must do this after the path searching (which ends up calling
     kpse_db_search recursively), so kpse->db.buckets stays NULL.  */
#ifdef PACKED_DB
  /* FlashTeX change (2026-10-09): ls-R's entries are packed (see
     packed_add); this table holds only the files inserted while running. */
  kpse->db = hash_create (ALIAS_HASH_SIZE);
#else
  kpse->db = hash_create (DB_HASH_SIZE);
#endif

  while (db_files && *db_files) {
    if (db_build (kpse, &(kpse->db), *db_files))
      ok = true;
    free (*db_files);
    db_files++;
  }

  if (!ok) {
    /* If db can't be built, leave `size' nonzero (so we don't
       rebuild it), but clear `buckets' (so we don't look in it).  */
    free (kpse->db.buckets);
    kpse->db.buckets = NULL;
    free (kpse->db.tails); /* FlashTeX change (2026-10-06): see hash.c.  */
    kpse->db.tails = NULL;
  }

  free (orig_db_files);

  /* Add the content of any alias databases.  There may exist more than
     one alias file along DB_NAME files.  This duplicates the above code
     -- should be a function.  */
  ok = false;
  db_files = kpathsea_all_path_search (kpse, db_path, ALIAS_NAME);
  orig_db_files = db_files;

  kpse->alias_db = hash_create (ALIAS_HASH_SIZE);

  while (db_files && *db_files) {
    if (alias_build (kpse, &(kpse->alias_db), *db_files))
      ok = true;
    free (*db_files);
    db_files++;
  }

  if (!ok) {
    free (kpse->alias_db.buckets);
    kpse->alias_db.buckets = NULL;
    free (kpse->alias_db.tails); /* FlashTeX change (2026-10-06): see hash.c.  */
    kpse->alias_db.tails = NULL;
  }

  free (orig_db_files);
}

/* Avoid doing anything if this PATH_ELT is irrelevant to the databases. */
str_list_type *
kpathsea_db_search (kpathsea kpse, const_string name,
                    const_string orig_path_elt, boolean all)
{
  const_string *db_dirs, *orig_dirs;
  const_string last_slash, path_elt;
  string temp_str = NULL;
  boolean done;
  unsigned e;
  str_list_type *ret = NULL;
  const_string *aliases, *r;
  boolean relevant = false;

  /* If we failed to build the database (or if this is the recursive
     call to build the db path), quit.  */
  if (kpse->db.buckets == NULL)
    return NULL;

  /* When tex-glyph.c calls us looking for, e.g., dpi600/cmr10.pk, we
     won't find it unless we change NAME to just `cmr10.pk' and append
     `/dpi600' to PATH_ELT.  We are justified in using a literal `/'
     here, since that's what tex-glyph.c unconditionally uses in
     DPI_BITMAP_SPEC.  But don't do anything if the / begins NAME; that
     should never happen.  */
  last_slash = strrchr (name, '/');
  if (last_slash && last_slash != name) {
    unsigned len = last_slash - name + 1;
    string dir_part = (string)xmalloc (len);
    strncpy (dir_part, name, len - 1);
    dir_part[len - 1] = 0;
    path_elt = temp_str = concat3 (orig_path_elt, "/", dir_part);
    name = last_slash + 1;
    free (dir_part);
  } else
    path_elt = orig_path_elt;

  /* Don't bother doing any lookups if this `path_elt' isn't covered by
     any of database directories.  We do this not so much because the
     extra couple of hash lookups matter -- they don't -- but rather
     because we want to return NULL in this case, so path_search can
     know to do a disk search.  */
  for (e = 0; !relevant && e < STR_LIST_LENGTH (kpse->db_dir_list); e++) {
    relevant = elt_in_db (STR_LIST_ELT (kpse->db_dir_list, e), path_elt);
  }
  if (!relevant)
    return NULL;

  /* If we have aliases for this name, use them.  */
  if (kpse->alias_db.buckets)
    aliases = hash_lookup (kpse->alias_db, name);
  else
    aliases = NULL;

  if (!aliases) {
    aliases = XTALLOC1 (const_string);
    aliases[0] = NULL;
  }
  {  /* Push aliases up by one and insert the original name at the front.  */
    unsigned i;
    unsigned len = 1; /* Have NULL element already allocated.  */
    for (r = aliases; *r; r++)
      len++;
    /* This is essentially
    XRETALLOC (aliases, len + 1, const_string);
       except that MSVC warns without the cast to `void *'.  */
    aliases = (const_string *) xrealloc ((void *) aliases,
                                         (len + 1) * sizeof(const_string));
    for (i = len; i > 0; i--) {
      aliases[i] = aliases[i - 1];
    }
    aliases[0] = name;
  }

  done = false;
  for (r = aliases; !done && *r; r++) {
    const_string ctry = *r;

    /* We have an ls-R db.  Look up `try'.  */
    orig_dirs = db_dirs = flashtex_db_lookup (kpse, ctry);

    ret = XTALLOC1 (str_list_type);
    *ret = str_list_init ();

    /* For each filename found, see if it matches the path element.  For
       example, if we have .../cx/cmr10.300pk and .../ricoh/cmr10.300pk,
       and the path looks like .../cx, we don't want the ricoh file.  */
    while (!done && db_dirs && *db_dirs) {
      string db_file = concat (*db_dirs, ctry);
      boolean matched = match (db_file, path_elt);

#ifdef KPSE_DEBUG
      if (KPATHSEA_DEBUG_P (KPSE_DEBUG_SEARCH))
        DEBUGF3 ("db:match(%s,%s) = %d\n", db_file, path_elt, matched);
#endif

      /* We got a hit in the database.  Now see if the file actually
         exists, possibly under an alias.  */
      if (matched) {
        string found = NULL;
        if (kpathsea_readable_file (kpse, db_file)) {
          found = db_file;

        } else {
          const_string *a;

          free (db_file); /* `db_file' wasn't on disk.  */

          /* The hit in the DB doesn't exist in disk.  Now try all its
             aliases.  For example, suppose we have a hierarchy on CD,
             thus `mf.bas', but ls-R contains `mf.base'.  Find it anyway.
             Could probably work around this with aliases, but
             this is pretty easy and shouldn't hurt.  The upshot is that
             if one of the aliases actually exists, we use that.  */
          for (a = aliases + 1; *a && !found; a++) {
            string atry = concat (*db_dirs, *a);
            if (kpathsea_readable_file (kpse, atry))
              found = atry;
            else
              free (atry);
          }
        }

        /* If we have a real file, add it to the list, maybe done.  */
        if (found) {
          str_list_add (ret, found);
          if (!all && found)
            done = true;
        }
      } else { /* no match in the db */
        free (db_file);
      }


      /* On to the next directory, if any.  */
      db_dirs++;
    }

    /* This is just the space for the pointers, not the strings.  */
    if (orig_dirs && *orig_dirs)
      free (orig_dirs);
  }

  free ((void *) aliases);

  /* If we had to break up NAME, free the TEMP_STR.  */
  if (temp_str)
    free (temp_str);

  return ret;
}

str_list_type *
kpathsea_db_search_list (kpathsea kpse, string* names,
                         const_string path_elt, boolean all)
{
  const_string *db_dirs, *orig_dirs;
  const_string last_slash, name, path;
  string temp_str = NULL;
  boolean done;
  unsigned e;
  const_string *aliases, *r;
  int n;
  str_list_type *ret = NULL;
  boolean relevant = false;

  /* If we failed to build the database (or if this is the recursive
     call to build the db path), quit.  */
  if (kpse->db.buckets == NULL)
    return NULL;

  /* Don't bother doing any lookups if this `path_elt' isn't covered by
     any of database directories.  We do this not so much because the
     extra couple of hash lookups matter -- they don't -- but rather
     because we want to return NULL in this case, so path_search can
     know to do a disk search.  */
  for (e = 0; !relevant && e < STR_LIST_LENGTH (kpse->db_dir_list); e++) {
    relevant = elt_in_db (STR_LIST_ELT (kpse->db_dir_list, e), path_elt);
  }
  if (!relevant)
    return NULL;

  done = false;
  ret = XTALLOC1 (str_list_type);
  *ret = str_list_init ();

  /* Handle each name. */
  for (n = 0; !done && names[n]; n++) {
      name = names[n];

      /* Absolute names should have been caught in our caller. */
      if (kpathsea_absolute_p(kpse, name, true))
          continue;

      /* When tex-glyph.c calls us looking for, e.g., dpi600/cmr10.pk, we
         won't find it unless we change NAME to just `cmr10.pk' and append
         `/dpi600' to PATH_ELT.  We are justified in using a literal `/'
         here, since that's what tex-glyph.c unconditionally uses in
         DPI_BITMAP_SPEC.  But don't do anything if the / begins NAME; that
         should never happen.  */
      last_slash = strrchr (name, '/');
      if (last_slash && last_slash != name) {
          unsigned len = last_slash - name + 1;
          string dir_part = (string)xmalloc (len);
          strncpy (dir_part, name, len - 1);
          dir_part[len - 1] = 0;
          path = temp_str = concat3 (path_elt, "/", dir_part);
          name = last_slash + 1;
          free (dir_part);
      } else {
          path = path_elt;
      }

      /* If we have aliases for this name, use them.  */
      if (kpse->alias_db.buckets)
          aliases = hash_lookup (kpse->alias_db, name);
      else
          aliases = NULL;

      if (!aliases) {
          aliases = XTALLOC1 (const_string);
          aliases[0] = NULL;
      }
      {  /* Push aliases up by one and insert the original name at front.  */
          unsigned i;
          unsigned len = 1; /* Have NULL element already allocated.  */
          for (r = aliases; *r; r++)
              len++;
          aliases = (const_string *) xrealloc ((void *) aliases,
                                               (len + 1) * sizeof(const_string));
          for (i = len; i > 0; i--) {
              aliases[i] = aliases[i - 1];
          }
          aliases[0] = name;
      }

      for (r = aliases; !done && *r; r++) {
          const_string ctry = *r;

          /* We have an ls-R db.  Look up `try'.  */
          orig_dirs = db_dirs = flashtex_db_lookup (kpse, ctry);

          /* For each filename found, see if it matches the path element.  For
             example, if we have .../cx/cmr10.300pk and .../ricoh/cmr10.300pk,
             and the path looks like .../cx, we don't want the ricoh file.  */
          while (!done && db_dirs && *db_dirs) {
            string db_file = concat (*db_dirs, ctry);
            boolean matched = match (db_file, path);

#ifdef KPSE_DEBUG
            if (KPATHSEA_DEBUG_P (KPSE_DEBUG_SEARCH))
              DEBUGF3 ("db:match(%s,%s) = %d\n", db_file, path, matched);
#endif

            /* We got a hit in the database.  Now see if the file actually
               exists, possibly under an alias.  */
            if (matched) {
              string found = NULL;
              if (kpathsea_readable_file (kpse, db_file)) {
                found = db_file;

              } else {
                const_string *a;

                free (db_file); /* `db_file' wasn't on disk.  */

                /* The hit in the DB doesn't exist in disk.  Now try all its
                   aliases.  For example, suppose we have a hierarchy on CD,
                   thus `mf.bas', but ls-R contains `mf.base'.  Find it anyway.
                   Could probably work around this with aliases, but
                   this is pretty easy and shouldn't hurt.  The upshot is that
                   if one of the aliases actually exists, we use that.  */
                for (a = aliases + 1; *a && !found; a++) {
                  string atry = concat (*db_dirs, *a);
                  if (kpathsea_readable_file (kpse, atry))
                    found = atry;
                  else
                    free (atry);
                }
              }

              /* If we have a real file, add it to the list, maybe done.  */
              if (found) {
                str_list_add (ret, found);
                if (!all)
                  done = true;
              }
            } else { /* no match in the db */
              free (db_file);
            }

            /* On to the next directory, if any.  */
            db_dirs++;
          }

          /* This is just the space for the pointers, not the strings.  */
          if (orig_dirs && *orig_dirs)
              free (orig_dirs);
      }

      free ((void *) aliases);
      if (temp_str)
          free (temp_str);
  }

  return ret;
}
