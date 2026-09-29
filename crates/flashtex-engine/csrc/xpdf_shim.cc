// A C interface to TeX Live's xpdf (third_party/xpdf, unmodified), for
// src/pdftex/pdftoepdf.rs, the port of pdfTeX's pdftoepdf.cc.
//
// pdftoepdf.cc calls xpdf's C++ classes directly. Its port calls the same
// member functions through the thin wrappers below, one per call it makes,
// so every parsing decision (xref repair, object streams, page-attribute
// inheritance, number lexing, font encodings) is xpdf's own.
//
// Every xpdf Object handed to Rust is a heap `Object` owned by the caller
// and released with ftx_obj_free. xpdf's accessors assume the object has the
// right type (a wrong one reads the wrong union member); pdftoepdf.cc checks
// types first, and where it does not, these wrappers check and return a
// neutral value instead of reading garbage, so that a malformed PDF can never
// crash the engine (DESIGN.md §4.5).
//
// Part of flashtex-engine, GPL-2.0-or-later.

#include <aconf.h>

#include <stdlib.h>
#include <string.h>

#include "GString.h"
#include "gmem.h"
#include "Object.h"
#include "Stream.h"
#include "Array.h"
#include "Dict.h"
#include "XRef.h"
#include "Catalog.h"
#include "Link.h"
#include "Page.h"
#include "GfxFont.h"
#include "PDFDoc.h"
#include "GlobalParams.h"

extern "C" {

// read_pdf_info's one-time initialisation.
void ftx_init(void) {
  if (!globalParams) {
    globalParams = new GlobalParams();
    globalParams->setErrQuiet(gFalse);
  }
}

// ---------------------------------------------------------------------------
// documents
// ---------------------------------------------------------------------------

// `new PDFDoc(new GString(file_name))`.
void *ftx_doc_open(const char *file_name) {
  GString *name = new GString(file_name);
  return new PDFDoc(name); // takes ownership of name
}

// `doc->isOk() && doc->okToPrint()`.
int ftx_doc_ok(void *d) {
  PDFDoc *doc = (PDFDoc *)d;
  return doc->isOk() && doc->okToPrint();
}

void ftx_doc_free(void *d) { delete (PDFDoc *)d; }

double ftx_doc_pdf_version(void *d) { return ((PDFDoc *)d)->getPDFVersion(); }

int ftx_doc_num_pages(void *d) {
  return ((PDFDoc *)d)->getCatalog()->getNumPages();
}

// The page of named destination `name`: -1 if there is no valid
// destination (`link == 0 || !link->isOk()`), else `findPage` of its page
// reference (0: not a page). pdfTeX reads `getPageRef()` even when the
// destination gives a page number, which leaves the reference unset; that
// case answers 0 here.
int ftx_doc_find_dest_page(void *d, const char *name) {
  PDFDoc *doc = (PDFDoc *)d;
  GString gname(name);
  LinkDest *link = doc->findDest(&gname);
  if (link == 0 || !link->isOk()) {
    delete link;
    return -1;
  }
  int page = 0;
  if (link->isPageRef()) {
    Ref ref = link->getPageRef();
    page = doc->getCatalog()->findPage(ref.num, ref.gen);
  }
  delete link;
  return page;
}

static Page *page_of(void *d, int n) {
  PDFDoc *doc = (PDFDoc *)d;
  Catalog *cat = doc->getCatalog();
  if (n < 1 || n > cat->getNumPages())
    return NULL;
  return cat->getPage(n);
}

// get_pagebox: box `which` (0 media, 1 crop, 2 bleed, 3 trim, 4 art) of
// page `n` into `out` (x1, y1, x2, y2). 0 if there is no such page.
int ftx_page_box(void *d, int n, int which, double *out) {
  Page *page = page_of(d, n);
  if (!page)
    return 0;
  PDFRectangle *r;
  switch (which) {
  case 0: r = page->getMediaBox(); break;
  case 1: r = page->getCropBox(); break;
  case 2: r = page->getBleedBox(); break;
  case 3: r = page->getTrimBox(); break;
  default: r = page->getArtBox(); break;
  }
  out[0] = r->x1;
  out[1] = r->y1;
  out[2] = r->x2;
  out[3] = r->y2;
  return 1;
}

int ftx_page_rotate(void *d, int n) {
  Page *page = page_of(d, n);
  return page ? page->getRotate() : 0;
}

// `page->getGroup()` as a dictionary object, or NULL.
Object *ftx_page_group(void *d, int n) {
  Page *page = page_of(d, n);
  if (!page || !page->getGroup())
    return NULL;
  Object *o = new Object();
  o->initDict(page->getGroup());
  return o;
}

// `page->getResourceDict()` as a dictionary object, or NULL.
Object *ftx_page_resources(void *d, int n) {
  Page *page = page_of(d, n);
  if (!page || !page->getResourceDict())
    return NULL;
  Object *o = new Object();
  o->initDict(page->getResourceDict());
  return o;
}

// `page->getContents(&obj)`.
Object *ftx_page_contents(void *d, int n) {
  Page *page = page_of(d, n);
  Object *o = new Object();
  if (page)
    page->getContents(o);
  else
    o->initNull();
  return o;
}

// `catalog->getPageRef(n)`: 0 if there is none.
int ftx_page_ref(void *d, int n, int *num, int *gen) {
  PDFDoc *doc = (PDFDoc *)d;
  if (!page_of(d, n))
    return 0;
  Ref *r = doc->getCatalog()->getPageRef(n);
  if (!r)
    return 0;
  *num = r->num;
  *gen = r->gen;
  return 1;
}

// `doc->getDocInfoNF(&obj)`.
Object *ftx_doc_info_nf(void *d) {
  Object *o = new Object();
  ((PDFDoc *)d)->getDocInfoNF(o);
  return o;
}

// `xref->fetch(num, gen, &obj)`.
Object *ftx_xref_fetch(void *d, int num, int gen) {
  Object *o = new Object();
  ((PDFDoc *)d)->getXRef()->fetch(num, gen, o);
  return o;
}

// ---------------------------------------------------------------------------
// objects
// ---------------------------------------------------------------------------

void ftx_obj_free(Object *o) {
  if (o) {
    o->free();
    delete o;
  }
}

int ftx_obj_type(Object *o) { return (int)o->getType(); }

const char *ftx_obj_type_name(Object *o) { return o->getTypeName(); }

int ftx_obj_bool(Object *o) { return o->isBool() ? o->getBool() : 0; }

int ftx_obj_int(Object *o) { return o->isInt() ? o->getInt() : 0; }

// `getNum()` (also `getReal()`, which is the same for a real).
double ftx_obj_num(Object *o) { return o->isNum() ? o->getNum() : 0.0; }

// The bytes of a string object (`getCString()`, `getLength()`).
const char *ftx_obj_string(Object *o, int *len) {
  if (!o->isString()) {
    *len = 0;
    return "";
  }
  *len = o->getString()->getLength();
  return o->getString()->getCString();
}

const char *ftx_obj_name(Object *o) { return o->isName() ? o->getName() : ""; }

void ftx_obj_ref(Object *o, int *num, int *gen) {
  if (o->isRef()) {
    *num = o->getRefNum();
    *gen = o->getRefGen();
  } else {
    *num = 0;
    *gen = 0;
  }
}

// `obj->fetch(xref, &out)`.
Object *ftx_obj_fetch(void *d, Object *o) {
  Object *r = new Object();
  o->fetch(((PDFDoc *)d)->getXRef(), r);
  return r;
}

int ftx_array_len(Object *o) { return o->isArray() ? o->arrayGetLength() : 0; }

Object *ftx_array_get_nf(Object *o, int i) {
  Object *r = new Object();
  if (o->isArray() && i >= 0 && i < o->arrayGetLength())
    o->arrayGetNF(i, r);
  else
    r->initNull();
  return r;
}

Object *ftx_array_get(Object *o, int i) {
  Object *r = new Object();
  if (o->isArray() && i >= 0 && i < o->arrayGetLength())
    o->arrayGet(i, r);
  else
    r->initNull();
  return r;
}

int ftx_dict_len(Object *o) { return o->isDict() ? o->dictGetLength() : 0; }

const char *ftx_dict_key(Object *o, int i) {
  if (!o->isDict() || i < 0 || i >= o->dictGetLength())
    return "";
  return o->dictGetKey(i);
}

Object *ftx_dict_val_nf(Object *o, int i) {
  Object *r = new Object();
  if (o->isDict() && i >= 0 && i < o->dictGetLength())
    o->dictGetValNF(i, r);
  else
    r->initNull();
  return r;
}

Object *ftx_dict_val(Object *o, int i) {
  Object *r = new Object();
  if (o->isDict() && i >= 0 && i < o->dictGetLength())
    o->dictGetVal(i, r);
  else
    r->initNull();
  return r;
}

Object *ftx_dict_lookup(Object *o, const char *key) {
  Object *r = new Object();
  if (o->isDict())
    o->dictLookup(key, r);
  else
    r->initNull();
  return r;
}

Object *ftx_dict_lookup_nf(Object *o, const char *key) {
  Object *r = new Object();
  if (o->isDict())
    o->dictLookupNF(key, r);
  else
    r->initNull();
  return r;
}

// pdftoepdf.cc's initDictFromDict: a new dictionary with the entries of
// dictionary `src` (or of stream `src`'s dictionary), unfetched.
Object *ftx_dict_copy(void *d, Object *src) {
  Dict *dict = src->isDict() ? src->getDict()
             : src->isStream() ? src->streamGetDict() : NULL;
  Object *r = new Object();
  r->initDict(((PDFDoc *)d)->getXRef());
  if (dict) {
    for (int i = 0, l = dict->getLength(); i < l; i++) {
      Object obj1;
      r->dictAdd(copyString(dict->getKey(i)), dict->getValNF(i, &obj1));
    }
  }
  return r;
}

// The dictionary of stream `o`, as a dictionary object (shared, not copied).
Object *ftx_stream_dict(Object *o) {
  Object *r = new Object();
  if (o->isStream())
    r->initDict(o->streamGetDict());
  else
    r->initNull();
  return r;
}

// copyStream's loop: `reset()`, then `getChar()` until EOF, of the stream
// itself (`raw` = 0: decoded through its filters) or of
// `getUndecodedStream()` (`raw` = 1). The bytes are malloc'ed; free them
// with ftx_free.
unsigned char *ftx_stream_bytes(Object *o, int raw, size_t *len) {
  *len = 0;
  if (!o->isStream())
    return NULL;
  Stream *str = raw ? o->getStream()->getUndecodedStream() : o->getStream();
  size_t cap = 4096, n = 0;
  unsigned char *buf = (unsigned char *)malloc(cap);
  if (!buf)
    return NULL;
  int c;
  str->reset();
  while ((c = str->getChar()) != EOF) {
    if (n == cap) {
      cap *= 2;
      unsigned char *nb = (unsigned char *)realloc(buf, cap);
      if (!nb) {
        free(buf);
        return NULL;
      }
      buf = nb;
    }
    buf[n++] = (unsigned char)c;
  }
  *len = n;
  return buf;
}

void ftx_free(void *p) { free(p); }

// ---------------------------------------------------------------------------
// fonts
// ---------------------------------------------------------------------------

// `GfxFont::makeFont(xref, tag, ref, fontdict->getDict())`; NULL if
// `fontdict` is not a dictionary.
void *ftx_font_make(void *d, const char *tag, int num, int gen, Object *fontdict) {
  if (!fontdict->isDict())
    return NULL;
  Ref ref;
  ref.num = num;
  ref.gen = gen;
  return GfxFont::makeFont(((PDFDoc *)d)->getXRef(), tag, ref,
                           fontdict->getDict());
}

int ftx_font_is_cid(void *f) { return ((GfxFont *)f)->isCIDFont(); }

// `((Gfx8BitFont *)font)->getCharName(i)`, or NULL.
const char *ftx_font_char_name(void *f, int i) {
  GfxFont *font = (GfxFont *)f;
  if (font->isCIDFont() || i < 0 || i > 255)
    return NULL;
  return ((Gfx8BitFont *)font)->getCharName(i);
}

void ftx_font_free(void *f) { delete (GfxFont *)f; }

} // extern "C"
