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

/* FlashTeX change (2026-10-09): ls-R as a packed index, mapped from a
   cache file.

   Each file of ls-R used to be one hash_element (key, value and next
   pointers: 24 bytes, 32 with the allocator's rounding, one malloc each)
   in a table of 64,007 buckets plus their tails, with the file names in
   the ls-R text read into memory: about 21 MB of every process's heap for
   TeX Live 2026's 248,000 files. Here each ls-R is one index image, a
   segment: 2^16 bucket heads, then an entry per file (three 32-bit
   numbers: the name's offset in the image's strings, its directory, the
   next entry of its bucket), each directory's offset, and the strings (the
   file names and the directories as `cur_dir' was, NUL-terminated). The
   image is written to a cache file (FLASHTEX: flashtex_lsr_cache_dir,
   keyed by the ls-R's path, checked by its size, modification time, inode
   and device) and mapped from there, read only: its pages are the file's,
   not the process's, and the next process maps it without reading ls-R.
   Without a cache the image stays in an anonymous mapping.

   Lookups answer exactly what hash_lookup (kpse->db, KEY) answered: a
   bucket's chain is in ls-R's order, as hash.c's is (an appended element
   was last), and the segments are searched in the order the ls-R files
   were read. Files inserted while running (kpathsea_db_insert) still go to
   `kpse->db', now a small hash table, and come after, as they were
   inserted after. The lines are read_line's (a line ends at LF, CR or
   CR LF, the last need not end, null bytes are dropped), the directories
   and the files kept are db_build's. Only where file names compare
   exactly (no MONOCASE_FILENAMES); elsewhere db_build is used as
   before.  */
#if !defined (MONOCASE_FILENAMES)
#define PACKED_DB 1
#endif

/* The cache directory (no cache when NULL) and the build salt its files
   are keyed by; see db.h. Defined everywhere, used only where there is a
   packed index with a cache.  */
char *flashtex_lsr_cache_dir = NULL;
char *flashtex_lsr_cache_salt = NULL;

#ifdef PACKED_DB
#if !defined (_WIN32)
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#define PACKED_CACHE 1
#endif

#define PACKED_NONE 0xffffffffu
#define PACKED_BUCKETS (1u << 16)
/* The parse's version: a change to how ls-R is read changes it (and the
   build salt, the program's build id, changes with every build).  */
#define PACKED_MAGIC "FTXLSR2\n"

typedef struct {
  unsigned key;   /* offset of the file name in the strings */
  unsigned dir;   /* index of its directory */
  unsigned next;  /* next entry of the bucket, or PACKED_NONE */
} packed_entry;

/* The image's header. The source's identity says which ls-R it indexes.  */
typedef struct {
  char magic[8];
  unsigned order;       /* 0x01020304: the writer's byte order */
  unsigned buckets;     /* PACKED_BUCKETS */
  unsigned n, ndirs, strings_len, pad;
  unsigned long long size, ino, dev;
  long long mtime_s, mtime_ns;
  unsigned long long salt;  /* packed_salt () of the program that wrote it */
} packed_header;

typedef struct {
  const unsigned *head;
  const packed_entry *ents;
  const unsigned *dir_off;
  const char *strings;
  unsigned n;
  void *image;          /* the mapping (or the allocation) */
  size_t image_len;
} packed_seg;

struct flashtex_packed_db {
  packed_seg *segs;
  unsigned nsegs;
};

/* FNV-1a, 64 bits, of S continuing from H.  */
static unsigned long long
packed_fnv64 (unsigned long long h, const char *s)
{
  for (; s && *s; s++)
    h = (h ^ (unsigned char) *s) * 1099511628211ull;
  return h;
}

/* The program's build salt (flashtex_lsr_cache_salt) and the parse's
   version, as one number.  */
static unsigned long long
packed_salt (void)
{
  return packed_fnv64 (packed_fnv64 (14695981039346656037ull, PACKED_MAGIC),
                       flashtex_lsr_cache_salt);
}

static unsigned
packed_hash (const_string key)
{
  unsigned h = 2166136261u;  /* FNV-1a */
  while (*key)
    h = (h ^ (unsigned char) *key++) * 16777619u;
  return h & (PACKED_BUCKETS - 1);
}

/* Memory that goes back to the system when freed: a mapping (a block this
   size freed to malloc stays in a macOS process).  */
static void *
packed_alloc (size_t len)
{
#ifdef PACKED_CACHE
  void *p = mmap (NULL, len ? len : 1, PROT_READ | PROT_WRITE,
                  MAP_PRIVATE | MAP_ANON, -1, 0);
  if (p == MAP_FAILED) {
    FATAL1 ("kpathsea: cannot map %lu bytes for ls-R", (unsigned long) len);
  }
  return p;
#else
  return xmalloc (len ? len : 1);
#endif
}

static void
packed_free (void *p, size_t len)
{
#ifdef PACKED_CACHE
  munmap (p, len ? len : 1);
#else
  (void) len;
  free (p);
#endif
}

static size_t
packed_image_len (unsigned n, unsigned ndirs, unsigned strings_len)
{
  return sizeof (packed_header) + PACKED_BUCKETS * sizeof (unsigned)
         + (size_t) n * sizeof (packed_entry)
         + (size_t) ndirs * sizeof (unsigned) + strings_len;
}

/* Point SEG into IMAGE (LEN bytes); false unless it is a well-formed image
   of the ls-R whose status is ST (when ST is not null).  Every offset is
   checked, and chains only go forward, so a damaged file cannot make a
   lookup read outside the image or loop.  */
static boolean
packed_attach (packed_seg *seg, void *image, size_t len, const struct stat *st)
{
  const packed_header *h = (const packed_header *) image;
  const char *base = (const char *) image;
  unsigned i;
  if (len < sizeof (packed_header) || memcmp (h->magic, PACKED_MAGIC, 8) != 0
      || h->order != 0x01020304u || h->buckets != PACKED_BUCKETS
      || h->strings_len == 0 || h->salt != packed_salt ()
      || packed_image_len (h->n, h->ndirs, h->strings_len) != len)
    return false;
#ifdef PACKED_CACHE
  if (st && (h->size != (unsigned long long) st->st_size
             || h->ino != (unsigned long long) st->st_ino
             || h->dev != (unsigned long long) st->st_dev
#if defined (__APPLE__)
             || h->mtime_s != (long long) st->st_mtimespec.tv_sec
             || h->mtime_ns != (long long) st->st_mtimespec.tv_nsec
#else
             || h->mtime_s != (long long) st->st_mtim.tv_sec
             || h->mtime_ns != (long long) st->st_mtim.tv_nsec
#endif
             ))
    return false;
#else
  (void) st;
#endif
  seg->head = (const unsigned *) (base + sizeof (packed_header));
  seg->ents = (const packed_entry *) (seg->head + PACKED_BUCKETS);
  seg->dir_off = (const unsigned *) (seg->ents + h->n);
  seg->strings = (const char *) (seg->dir_off + h->ndirs);
  seg->n = h->n;
  if (seg->strings[h->strings_len - 1] != 0)
    return false;
  for (i = 0; i < PACKED_BUCKETS; i++)
    if (seg->head[i] != PACKED_NONE && seg->head[i] >= h->n)
      return false;
  for (i = 0; i < h->n; i++) {
    const packed_entry *e = &seg->ents[i];
    if (e->key >= h->strings_len || e->dir >= h->ndirs
        || (e->next != PACKED_NONE && (e->next <= i || e->next >= h->n)))
      return false;
  }
  for (i = 0; i < h->ndirs; i++)
    if (seg->dir_off[i] >= h->strings_len)
      return false;
  seg->image = image;
  seg->image_len = len;
  return true;
}

#ifdef PACKED_CACHE
/* The cache file of the ls-R at PATH (malloc'd), or NULL.  */
static string
packed_cache_file (const_string path)
{
  unsigned long long h;
  char name[40];
  if (!flashtex_lsr_cache_dir || !*flashtex_lsr_cache_dir)
    return NULL;
  h = packed_fnv64 (packed_salt (), path);
  snprintf (name, sizeof name, "/lsr-%016llx.idx", h);
  return concat (flashtex_lsr_cache_dir, name);
}

/* SEG from the cache file of the ls-R at PATH whose status is ST. The
   file's size, as fstat gives it now, must be the image's own (a file cut
   short is a miss, not a read past its end). The writer replaces a cache
   file by a rename, which leaves a mapping of the old one intact; only a
   file shortened in place by another program after this check could still
   end a lookup with SIGBUS.  */
static boolean
packed_load (packed_seg *seg, const_string path, const struct stat *st)
{
  string file = packed_cache_file (path);
  struct stat cs;
  void *image;
  int fd;
  if (!file)
    return false;
  fd = open (file, O_RDONLY);
  free (file);
  if (fd < 0)
    return false;
  if (fstat (fd, &cs) != 0 || cs.st_size <= 0) {
    close (fd);
    return false;
  }
  image = mmap (NULL, (size_t) cs.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
  close (fd);
  if (image == MAP_FAILED)
    return false;
  if (!packed_attach (seg, image, (size_t) cs.st_size, st)) {
    munmap (image, (size_t) cs.st_size);
    return false;
  }
  return true;
}

/* Write IMAGE (LEN bytes) as the cache file of the ls-R at PATH, and map
   it: SEG then points into the file. False (SEG unchanged) if it could
   not be written, or if ls-R's modification time is under two seconds
   old (a change within its time stamp's granularity would go unseen).  */
static boolean
packed_store (packed_seg *seg, const_string path, const struct stat *st,
              void *image, size_t len)
{
  string file, tmp;
  const char *p = (const char *) image;
  size_t left = len;
  int fd;
  boolean ok = false;
  if (time (NULL) - st->st_mtime < 2)
    return false;
  file = packed_cache_file (path);
  if (!file)
    return false;
  mkdir (flashtex_lsr_cache_dir, 0777);
  /* a name no other writer (process or thread) can have */
  tmp = concat (file, ".XXXXXX");
  fd = mkstemp (tmp);
  if (fd >= 0) {
    fchmod (fd, 0644);
    while (left > 0) {
      ssize_t w = write (fd, p, left);
      if (w <= 0)
        break;
      p += w;
      left -= (size_t) w;
    }
    ok = left == 0;
    if (close (fd) != 0)
      ok = false;
    if (ok)
      ok = rename (tmp, file) == 0;
    if (!ok)
      unlink (tmp);
  }
  free (tmp);
  free (file);
  if (!ok)
    return false;
  {
    packed_seg mapped;
    if (!packed_load (&mapped, path, st))
      return false;
    *seg = mapped;
  }
  return true;
}
#endif /* PACKED_CACHE */

/* The ls-R at DB_FILENAME read into SEG: its lines read_line's and its
   entries db_build's (see above).  TOP_DIR is its directory.  False if it
   cannot be opened.  */
static boolean
packed_read (kpathsea kpse, packed_seg *seg, const_string db_filename,
             const_string top_dir, struct stat *st, boolean have_st)
{
  FILE *db_file = fopen (db_filename, FOPEN_R_MODE);
  size_t buf_size = 0, buf_cap = 1 << 20, got, top_len = strlen (top_dir);
  string buf, line, q, w, end;
  unsigned n = 0, ndirs = 0, b, i, d, s;
  size_t strings_len = 0, len;
  boolean in_dir;
  char *image, *strings;
  unsigned *head, *tail, *dir_off;
  packed_entry *ents;
  packed_header *h;

  if (!db_file)
    return false;
  if (have_st && st->st_size > 0)
    buf_cap = (size_t) st->st_size + 1;
  buf = (string) packed_alloc (buf_cap + 1);
  while ((got = fread (buf + buf_size, 1, buf_cap - buf_size, db_file)) > 0) {
    buf_size += got;
    if (buf_size == buf_cap) {  /* longer than it was: grow */
      string more = (string) packed_alloc (2 * buf_cap + 1);
      memcpy (more, buf, buf_size);
      packed_free (buf, buf_cap + 1);
      buf = more;
      buf_cap *= 2;
    }
  }
  xfclose (db_file, db_filename);

  /* The lines, NUL-terminated, packed to the front of the buffer: each is
     no longer than the text it came from.  */
  end = buf;
  for (q = buf; q < buf + buf_size; ) {
    for (w = end; q < buf + buf_size && *q != '\n' && *q != '\r'; q++)
      if (*q != 0)
        *w++ = *q;
    if (q < buf + buf_size && *q == '\r' && q + 1 < buf + buf_size && q[1] == '\n')
      q += 2;
    else if (q < buf + buf_size)
      q++;
    *w++ = 0;
    end = w;
  }

  /* Count, then fill. A line like `/foo:' (or `./foo:') = new directory
     foo; a file line counts only after a directory not ignored, and not
     when blank, `.' or `..'.  */
  for (in_dir = false, line = buf; line < end; line += len + 1) {
    len = strlen (line);
    if (len > 0 && line[len - 1] == ':' && kpathsea_absolute_p (kpse, line, true)) {
      in_dir = !ignore_dir_p (line);
      if (in_dir) {
        ndirs++;
        strings_len += (*line == '.' ? top_len + len - 2 : len) + 1;
      }
    } else if (*line != 0 && in_dir
               && !(*line == '.' && (line[1] == 0 || (line[1] == '.' && line[2] == 0)))) {
      n++;
      strings_len += len + 1;
    }
  }
  if (strings_len == 0)
    strings_len = 1;
  if (strings_len > 0xfffffff0u || n > 0xfffffff0u) {
    packed_free (buf, buf_cap + 1);
    FATAL1 ("kpathsea: %s: too large", db_filename);
  }
  image = (char *) packed_alloc (packed_image_len (n, ndirs, (unsigned) strings_len));
  h = (packed_header *) image;
  memset (h, 0, sizeof *h);
  memcpy (h->magic, PACKED_MAGIC, 8);
  h->salt = packed_salt ();
  h->order = 0x01020304u;
  h->buckets = PACKED_BUCKETS;
  h->n = n;
  h->ndirs = ndirs;
  h->strings_len = (unsigned) strings_len;
  if (have_st) {
    h->size = (unsigned long long) st->st_size;
    h->ino = (unsigned long long) st->st_ino;
    h->dev = (unsigned long long) st->st_dev;
#if defined (__APPLE__)
    h->mtime_s = (long long) st->st_mtimespec.tv_sec;
    h->mtime_ns = (long long) st->st_mtimespec.tv_nsec;
#elif !defined (_WIN32)
    h->mtime_s = (long long) st->st_mtim.tv_sec;
    h->mtime_ns = (long long) st->st_mtim.tv_nsec;
#endif
  }
  head = (unsigned *) (image + sizeof (packed_header));
  ents = (packed_entry *) (head + PACKED_BUCKETS);
  dir_off = (unsigned *) (ents + n);
  strings = (char *) (dir_off + ndirs);
  strings[strings_len - 1] = 0;
  tail = (unsigned *) packed_alloc (PACKED_BUCKETS * sizeof (unsigned));
  for (b = 0; b < PACKED_BUCKETS; b++)
    head[b] = tail[b] = PACKED_NONE;
  for (in_dir = false, i = 0, d = 0, s = 0, line = buf; line < end; line += len + 1) {
    len = strlen (line);
    if (len > 0 && line[len - 1] == ':' && kpathsea_absolute_p (kpse, line, true)) {
      in_dir = !ignore_dir_p (line);
      if (in_dir) {
        /* cur_dir: the name with DIR_SEP for the colon, `./' replaced by
           TOP_DIR.  */
        dir_off[d++] = s;
        if (*line == '.') {
          memcpy (strings + s, top_dir, top_len);
          memcpy (strings + s + top_len, line + 2, len - 3);
          s += (unsigned) (top_len + len - 3);
        } else {
          memcpy (strings + s, line, len - 1);
          s += (unsigned) (len - 1);
        }
        strings[s++] = DIR_SEP;
        strings[s++] = 0;
      }
    } else if (*line != 0 && in_dir
               && !(*line == '.' && (line[1] == 0 || (line[1] == '.' && line[2] == 0)))) {
      b = packed_hash (line);
      ents[i].key = s;
      ents[i].dir = d - 1;
      ents[i].next = PACKED_NONE;
      if (tail[b] == PACKED_NONE)
        head[b] = i;
      else
        ents[tail[b]].next = i;
      tail[b] = i;
      i++;
      memcpy (strings + s, line, len + 1);
      s += (unsigned) (len + 1);
    }
  }
  packed_free (tail, PACKED_BUCKETS * sizeof (unsigned));
  packed_free (buf, buf_cap + 1);
  seg->head = head;
  seg->ents = ents;
  seg->dir_off = dir_off;
  seg->strings = strings;
  seg->n = n;
  seg->image = image;
  seg->image_len = packed_image_len (n, ndirs, (unsigned) strings_len);
  return true;
}

/* db_build for a packed index (see above): add the ls-R at DB_FILENAME, from
   its cache file or read; true if it had entries.  */
static boolean
packed_build (kpathsea kpse, const_string db_filename)
{
  unsigned len = strlen (db_filename) - sizeof (DB_NAME) + 1; /* Keep the /. */
  string top_dir = (string) xmalloc (len + 1);
  struct flashtex_packed_db *p = kpse->flashtex_packed_db;
  packed_seg seg;
  struct stat st;
  boolean have_st = stat (db_filename, &st) == 0;
  boolean ok = false;

  strncpy (top_dir, db_filename, len);
  top_dir[len] = 0;
  memset (&seg, 0, sizeof seg);
#ifdef PACKED_CACHE
  if (have_st)
    ok = packed_load (&seg, db_filename, &st);
#endif
  if (!ok) {
    if (!packed_read (kpse, &seg, db_filename, top_dir, &st, have_st)) {
      free (top_dir);
      return false;
    }
    if (seg.n == 0) {
      packed_free (seg.image, seg.image_len);
      WARNING1 ("kpathsea: %s: No usable entries in ls-R", db_filename);
      WARNING ("kpathsea: See the manual for how to generate ls-R");
      free (top_dir);
      return false;
    }
#ifdef PACKED_CACHE
    if (have_st) {
      void *image = seg.image;
      size_t image_len = seg.image_len;
      if (packed_store (&seg, db_filename, &st, image, image_len))
        packed_free (image, image_len);
    }
#endif
  }
  if (!p) {
    p = (struct flashtex_packed_db *) xcalloc (1, sizeof (*p));
    kpse->flashtex_packed_db = p;
  }
  p->segs = (packed_seg *) xrealloc (p->segs, (p->nsegs + 1) * sizeof (packed_seg));
  p->segs[p->nsegs++] = seg;
  str_list_add (&(kpse->db_dir_list), xstrdup (top_dir));
  free (top_dir);
  return true;
}
#endif /* PACKED_DB */

/* FlashTeX change (2026-10-09): the packed index's segments given back
   (kpathsea_finish): mappings unmapped, buffers freed.  */
void
flashtex_db_free (kpathsea kpse)
{
#ifdef PACKED_DB
  struct flashtex_packed_db *p = kpse->flashtex_packed_db;
  unsigned g;
  if (!p)
    return;
  for (g = 0; g < p->nsegs; g++)
    packed_free (p->segs[g].image, p->segs[g].image_len);
  free (p->segs);
  free (p);
  kpse->flashtex_packed_db = NULL;
#else
  (void) kpse;
#endif
}

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
  unsigned g, i, b;
  if (!p)
    return hash_lookup (kpse->db, key);
  ret = cstr_list_init ();
  b = packed_hash (key);
  for (g = 0; g < p->nsegs; g++) {
    const packed_seg *seg = &p->segs[g];
    for (i = seg->head[b]; i != PACKED_NONE; i = seg->ents[i].next)
      if (STREQ (key, seg->strings + seg->ents[i].key))
        cstr_list_add (&ret, seg->strings + seg->dir_off[seg->ents[i].dir]);
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

#ifndef PACKED_DB
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
    string buf = (string) xmalloc (buf_cap + 1);
    string next, buf_end;
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
        hash_insert_normalized (table, line, cur_dir);
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
#endif /* !PACKED_DB */


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
  /* FlashTeX change (2026-10-09): ls-R's entries are packed (packed_build);
     this table holds only the files inserted while running.  */
  kpse->db = hash_create (ALIAS_HASH_SIZE);
#else
  kpse->db = hash_create (DB_HASH_SIZE);
#endif

  while (db_files && *db_files) {
#ifdef PACKED_DB
    if (packed_build (kpse, *db_files))
#else
    if (db_build (kpse, &(kpse->db), *db_files))
#endif
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
