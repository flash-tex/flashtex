/* Probe for the FFI of src/lib.rs (FlashTeX, GPL-2.0-or-later).
 *
 * Compiled by build.rs against the vendored ICU headers. It reports the
 * size of every type the Rust FFI declares and the value of every constant
 * it copies, so that the crate's tests can compare them with the Rust
 * definitions, and the version-suffixed name of one function, which the
 * FFI's link names assume.
 */
#include <unicode/utypes.h>
#include <unicode/uversion.h>
#include <unicode/ubidi.h>
#include <unicode/ucnv.h>
#include <unicode/ubrk.h>

typedef struct {
  const char *name;
  long long value;
} flashtex_icu_probe_entry;

#define SIZE(T) {"sizeof " #T, (long long)sizeof(T)}
#define VAL(c) {#c, (long long)(c)}

static const flashtex_icu_probe_entry entries[] = {
  SIZE(UChar), SIZE(UErrorCode), SIZE(UBiDiLevel), SIZE(UBiDiDirection),
  SIZE(UConverterType), SIZE(UBreakIteratorType), SIZE(UVersionInfo),
  VAL(U_ICU_VERSION_MAJOR_NUM), VAL(U_ICU_VERSION_MINOR_NUM),
  VAL(U_ICU_VERSION_PATCHLEVEL_NUM),
  VAL(U_ZERO_ERROR), VAL(U_FILE_ACCESS_ERROR), VAL(U_STRING_NOT_TERMINATED_WARNING),
  VAL(UBIDI_DEFAULT_LTR), VAL(UBIDI_DEFAULT_RTL),
  VAL(UBIDI_LTR), VAL(UBIDI_RTL), VAL(UBIDI_MIXED), VAL(UBIDI_NEUTRAL),
  VAL(UCNV_UTF32_BigEndian), VAL(UCNV_UTF32_LittleEndian),
  VAL(UBRK_LINE), VAL(UBRK_DONE),
};

/* The probe's own name is not renamed (it is not ICU's). */
const flashtex_icu_probe_entry *flashtex_icu_probe(size_t *count) {
  *count = sizeof entries / sizeof entries[0];
  return entries;
}

/* "ubidi_open_78": the name urename.h gives ubidi_open. */
const char *flashtex_icu_renamed_ubidi_open(void) {
#define FLASHTEX_STR2(x) #x
#define FLASHTEX_STR(x) FLASHTEX_STR2(x)
  return FLASHTEX_STR(ubidi_open);
}
