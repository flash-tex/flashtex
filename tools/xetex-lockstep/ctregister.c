// fontmatch.py --font-dirs: make the font files of the directories in
// $XETEX_EXTRA_FONT_DIRS (colon separated) visible to TeX Live's macOS xetex,
// which finds fonts by name through Core Text only, without installing them.
// Loaded into a re-signed copy of xetex with DYLD_INSERT_LIBRARIES; before
// main runs it registers every .otf, .ttf and .ttc file with
// CTFontManagerRegisterFontsForURL at process scope, so Core Text (and
// NSFontManager) list them as they would installed fonts, for that process
// only. The oracle's code is unchanged; nothing outside the process changes.
// MIT, like the rest of tools/.
#include <CoreFoundation/CoreFoundation.h>
#include <CoreText/CoreText.h>
#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

static void register_dir(const char *d) {
  DIR *dir = opendir(d);
  if (!dir) {
    fprintf(stderr, "ctregister: cannot read %s\n", d);
    return;
  }
  struct dirent *e;
  while ((e = readdir(dir))) {
    const char *n = e->d_name;
    size_t l = strlen(n);
    if (l < 4)
      continue;
    const char *x = n + l - 4;
    if (strcasecmp(x, ".otf") && strcasecmp(x, ".ttf") && strcasecmp(x, ".ttc"))
      continue;
    char p[4096];
    snprintf(p, sizeof p, "%s/%s", d, n);
    CFURLRef u = CFURLCreateFromFileSystemRepresentation(NULL, (const UInt8 *)p, (CFIndex)strlen(p), false);
    CFErrorRef err = NULL;
    if (!CTFontManagerRegisterFontsForURL(u, kCTFontManagerScopeProcess, &err)) {
      fprintf(stderr, "ctregister: not registered: %s\n", p);
      if (err)
        CFRelease(err);
    }
    CFRelease(u);
  }
  closedir(dir);
}

__attribute__((constructor)) static void ctregister_init(void) {
  const char *v = getenv("XETEX_EXTRA_FONT_DIRS");
  if (!v)
    return;
  char *s = strdup(v), *save = NULL;
  for (char *t = strtok_r(s, ":", &save); t; t = strtok_r(NULL, ":", &save))
    register_dir(t);
  free(s);
}
