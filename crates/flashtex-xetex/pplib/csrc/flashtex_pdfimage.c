/* XeTeX's pdfimage.cpp (texk/web2c/xetexdir, TeX Live 2026; MIT, copyright
 * (c) 1994-2008 SIL International, 2009-2012 Jonathan Kew, 2012-2015
 * Khaled Hosny), as C over the same pplib: `pdf_get_rect` and
 * `pdf_count_pages`, statement for statement.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the
 * "Software"), to deal in the Software without restriction, including
 * without limitation the rights to use, copy, modify, merge, publish,
 * distribute, sublicense, and/or sell copies of the Software, and to permit
 * persons to whom the Software is furnished to do so, subject to the
 * following conditions:
 *
 * The above copyright notice and this permission notice shall be included
 * in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
 * OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
 * NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR
 * OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
 * ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
 * OTHER DEALINGS IN THE SOFTWARE.
 *
 * Two differences from the C++, both where XeTeX itself has no result:
 *
 * - `pdf_get_rect` dereferences `ppdoc_page`'s result and takes its object
 *   for a dictionary without a test. For a document whose page cannot be
 *   found (every cross-reference offset wrong) or whose page object is not
 *   a dictionary, XeTeX 0.999998 crashes (SIGSEGV; measured on TeX Live
 *   2026 x86_64-linux, flash-tex/flashtex#2494). Here both return -1: the
 *   picture cannot be loaded.
 * - The C++ returns -1 for a page without any box before `ppdoc_free`, so
 *   it leaks the document; here it is freed (no difference in results).
 *
 * The box is written as doubles: the caller stores them into `float`s as
 * XeTeX's `realrect` does (crates/flashtex-xetex/src/pic.rs, `round_f32`).
 */

#include <math.h>

#include "ppapi.h"

/* XeTeX_ext.h */
#define pdfbox_crop 1
#define pdfbox_media 2
#define pdfbox_bleed 3
#define pdfbox_trim 4
#define pdfbox_art 5

/* "use our own fmin function because it seems to be missing on certain
 * platforms" (pdfimage.cpp) */
static double
my_fmin(double x, double y)
{
	return (x < y) ? x : y;
}

/* The box of page `page_num` converted to TeX points: x, y, wd, ht.
 * 0, or -1 as XeTeX's. */
int
flashtex_pdf_get_rect(const char* filename, int page_num, int pdf_box, double* box)
{
	ppdoc*	doc = ppdoc_load(filename);

	if (!doc) {
		return -1;
	}

	int	pages = ppdoc_page_count(doc);

	if (page_num > pages)
		page_num = pages;
	if (page_num < 0)
		page_num = pages + 1 + page_num;
	if (page_num < 1)
		page_num = 1;

	ppref*	page = ppdoc_page(doc, page_num);
	if (!page || page->object.type != PPDICT) {
		/* XeTeX crashes here (see the header) */
		ppdoc_free(doc);
		return -1;
	}
	ppdict*	pdict = page->object.dict;

	pprect	Rect;
	pprect*	r;

	switch (pdf_box) {
		default:
		case pdfbox_crop:
			r = ppdict_get_box(pdict, "CropBox", &Rect);
			break;
		case pdfbox_media:
			r = ppdict_get_box(pdict, "MediaBox", &Rect);
			break;
		case pdfbox_bleed:
			r = ppdict_get_box(pdict, "BleedBox", &Rect);
			break;
		case pdfbox_trim:
			r = ppdict_get_box(pdict, "TrimBox", &Rect);
			break;
		case pdfbox_art:
			r = ppdict_get_box(pdict, "ArtBox", &Rect);
			break;
	}

/*
 *  In pplib, r can be NULL. If r == NULL, we try "CropBox",
 *  "MediaBox",  "BleedBox", "TrimBox", "ArtBox" in this order.
 */
	if (!r) {
		r = ppdict_get_box(pdict, "CropBox", &Rect);
	}
	if (!r) {
		r = ppdict_get_box(pdict, "MediaBox", &Rect);
	}
	if (!r) {
		r = ppdict_get_box(pdict, "BleedBox", &Rect);
	}
	if (!r) {
		r = ppdict_get_box(pdict, "TrimBox", &Rect);
	}
	if (!r) {
		r = ppdict_get_box(pdict, "ArtBox", &Rect);
	}

/*
 * If r == NULL, return error.
 */
	if (!r) {
		ppdoc_free(doc);
		return -1;
	}

	ppint RotAngle = 0;
	(void)ppdict_get_int(pdict, "Rotate", &RotAngle);
	RotAngle = RotAngle % 360;
	if (RotAngle < 0)
		RotAngle += 360;
	if (RotAngle == 90 || RotAngle == 270) {
		box[2] = 72.27 / 72 * fabs(r->ry - r->ly);
		box[3] = 72.27 / 72 * fabs(r->rx - r->lx);
	} else {
		box[2] = 72.27 / 72 * fabs(r->rx - r->lx);
		box[3] = 72.27 / 72 * fabs(r->ry - r->ly);
	}
	box[0] = 72.27 / 72 * my_fmin(r->lx, r->rx);
	box[1] = 72.27 / 72 * my_fmin(r->ly, r->ry);

	ppdoc_free(doc);

	return 0;
}

int
flashtex_pdf_count_pages(const char* filename)
{
	int	pages = 0;
	ppdoc*	doc = ppdoc_load(filename);

	if (!doc) {
		return 0;
	}

	pages = ppdoc_page_count(doc);
	if (doc) ppdoc_free(doc);

	return pages;
}

/* pplib's version string (ppapi.h's `pplib_version`), for the pin test. */
const char*
flashtex_pplib_version(void)
{
	return pplib_version;
}
