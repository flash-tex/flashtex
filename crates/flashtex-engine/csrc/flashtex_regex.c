/* flashtex_regex.c -- the POSIX regular expressions pdfTeX's \pdfmatch uses.

   utils.c's matchstrings() calls regcomp(3) and regexec(3) from the C
   library's <regex.h> on every platform but Windows (pdftexdir/regex, a
   copy of glibc's, is compiled only for MINGW32; see am/libpdftex.am). So
   the implementation that defines \pdfmatch is the platform's own, and this
   file calls exactly that, with utils.c's flags. regex_t and regmatch_t
   differ between C libraries, which is why this is C and not Rust FFI.

   Part of flashtex-engine (GPL-2.0-or-later). */

#include <regex.h>
#include <stdlib.h>
#include <string.h>

/* Compile PATTERN (REG_EXTENDED, plus REG_ICASE if ICASE) and match it
   against STR with up to NMATCH subexpressions. Returns 1 on a match, 0
   when there is none, and -1 when PATTERN does not compile, with
   regerror's message in ERRBUF (at most ERRLEN bytes, NUL-terminated).
   SO[i] and EO[i] receive pmatch[i].rm_so and rm_eo. */
int flashtex_regex_match(const char *pattern, const char *str, int icase, int nmatch,
                         long long *so, long long *eo, char *errbuf, size_t errlen)
{
    regex_t preg;
    regmatch_t *pmatch = NULL;
    int cflags = REG_EXTENDED;
    int ret, i, result;

    if (icase)
        cflags |= REG_ICASE;
    ret = regcomp(&preg, pattern, cflags);
    if (ret != 0) {
        regerror(ret, &preg, errbuf, errlen);
        regfree(&preg);
        return -1;
    }
    if (nmatch > 0) {
        pmatch = calloc((size_t) nmatch, sizeof(regmatch_t));
        if (!pmatch) {
            regfree(&preg);
            return 0;
        }
    }
    ret = regexec(&preg, str, (size_t) nmatch, pmatch, 0);
    result = (ret == 0) ? 1 : 0;
    for (i = 0; i < nmatch; i++) {
        so[i] = (long long) pmatch[i].rm_so;
        eo[i] = (long long) pmatch[i].rm_eo;
    }
    free(pmatch);
    regfree(&preg);
    return result;
}
