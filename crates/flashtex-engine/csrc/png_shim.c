/* A C interface to TeX Live's libpng (third_party/libpng, unmodified), for
 * src/pdftex/writepng.rs, the port of pdfTeX's writepng.c.
 *
 * libpng reports errors by longjmp to the caller's setjmp, which Rust code
 * must never be jumped across. So every libpng call that can fail is made
 * here, under its own setjmp, and a failure comes back as a return value of
 * -1; the port then stops the run with pdfTeX's message ("libpng: internal
 * error"). Warnings are dropped, as writepng.c's `warn` drops them; errors
 * print libpng's own message on stderr first (pdfTeX passes no error
 * function, so libpng's default one prints it).
 *
 * Part of flashtex-engine, GPL-2.0-or-later. */

#include <stdio.h>
#include <stdlib.h>
#include <png.h>

typedef struct {
    png_structp png;
    png_infop info;
    FILE *file;
} ftpng;

static void warn(png_structp png_ptr, png_const_charp msg)
{
    (void) png_ptr;
    (void) msg;
}

void ftpng_close(ftpng *p)
{
    if (!p)
        return;
    if (p->png)
        png_destroy_read_struct(&p->png, p->info ? &p->info : NULL, NULL);
    if (p->file)
        fclose(p->file);
    free(p);
}

/* read_png_info up to png_read_info. `*err`: 0 ok, 1 the file cannot be
 * opened, 2 png_create_read_struct failed, 3 png_create_info_struct failed,
 * 4 libpng error. */
ftpng *ftpng_open(const char *name, int *err)
{
    ftpng *p = (ftpng *) calloc(1, sizeof(ftpng));
    if (!p) {
        *err = 2;
        return NULL;
    }
    p->file = fopen(name, "rb");
    if (!p->file) {
        *err = 1;
        ftpng_close(p);
        return NULL;
    }
    if ((p->png = png_create_read_struct(PNG_LIBPNG_VER_STRING,
                                         NULL, NULL, warn)) == NULL) {
        *err = 2;
        ftpng_close(p);
        return NULL;
    }
    if ((p->info = png_create_info_struct(p->png)) == NULL) {
        *err = 3;
        ftpng_close(p);
        return NULL;
    }
    if (setjmp(png_jmpbuf(p->png))) {
        *err = 4;
        ftpng_close(p);
        return NULL;
    }
#if PNG_LIBPNG_VER >= 10603
    /* ignore possibly incorrect CMF bytes */
    png_set_option(p->png, PNG_MAXIMUM_INFLATE_WINDOW, PNG_OPTION_ON);
#endif
    png_init_io(p->png, p->file);
    png_read_info(p->png, p->info);
    *err = 0;
    return p;
}

/* The png_get_* queries writepng.c makes (none of them fails). */
enum {
    FTPNG_WIDTH, FTPNG_HEIGHT, FTPNG_BIT_DEPTH, FTPNG_COLOR_TYPE,
    FTPNG_INTERLACE_TYPE, FTPNG_ROWBYTES, FTPNG_X_PPM, FTPNG_Y_PPM
};

unsigned long ftpng_get(ftpng *p, int what)
{
    switch (what) {
    case FTPNG_WIDTH: return png_get_image_width(p->png, p->info);
    case FTPNG_HEIGHT: return png_get_image_height(p->png, p->info);
    case FTPNG_BIT_DEPTH: return png_get_bit_depth(p->png, p->info);
    case FTPNG_COLOR_TYPE: return png_get_color_type(p->png, p->info);
    case FTPNG_INTERLACE_TYPE: return png_get_interlace_type(p->png, p->info);
    case FTPNG_ROWBYTES: return png_get_rowbytes(p->png, p->info);
    case FTPNG_X_PPM: return png_get_x_pixels_per_meter(p->png, p->info);
    case FTPNG_Y_PPM: return png_get_y_pixels_per_meter(p->png, p->info);
    default: return 0;
    }
}

unsigned long ftpng_valid(ftpng *p, unsigned long flag)
{
    return png_get_valid(p->png, p->info, (png_uint_32) flag);
}

/* png_get_PLTE: the palette as RGB triples; the count in *num (0 if none). */
const png_color *ftpng_plte(ftpng *p, int *num)
{
    png_colorp palette = NULL;
    *num = 0;
    if (png_get_PLTE(p->png, p->info, &palette, num) == 0) {
        *num = 0;
        return NULL;
    }
    return palette;
}

/* png_get_gAMA and png_get_gAMA_fixed. */
void ftpng_gamma(ftpng *p, double *gamma, long long *fixed)
{
    png_fixed_point f = 0;
    png_get_gAMA(p->png, p->info, gamma);
    png_get_gAMA_fixed(p->png, p->info, &f);
    *fixed = f;
}

/* The transformations of write_png, then png_read_update_info. `what`
 * selects one; `a` and `b` are png_set_gamma's arguments. */
enum {
    FTPNG_TRNS_TO_ALPHA, FTPNG_STRIP_ALPHA, FTPNG_STRIP_16, FTPNG_SET_GAMMA,
    FTPNG_INTERLACE_HANDLING, FTPNG_UPDATE_INFO
};

int ftpng_transform(ftpng *p, int what, double a, double b)
{
    if (setjmp(png_jmpbuf(p->png)))
        return -1;
    switch (what) {
    case FTPNG_TRNS_TO_ALPHA: png_set_tRNS_to_alpha(p->png); break;
    case FTPNG_STRIP_ALPHA: png_set_strip_alpha(p->png); break;
    case FTPNG_STRIP_16: png_set_strip_16(p->png); break;
    case FTPNG_SET_GAMMA: png_set_gamma(p->png, a, b); break;
    case FTPNG_INTERLACE_HANDLING: (void) png_set_interlace_handling(p->png); break;
    case FTPNG_UPDATE_INFO: png_read_update_info(p->png, p->info); break;
    default: break;
    }
    return 0;
}

/* png_read_row into `row` (rowbytes long). */
int ftpng_read_row(ftpng *p, unsigned char *row)
{
    if (setjmp(png_jmpbuf(p->png)))
        return -1;
    png_read_row(p->png, row, NULL);
    return 0;
}

/* png_read_image into `rows` (height pointers to rowbytes each). */
int ftpng_read_image(ftpng *p, unsigned char **rows)
{
    if (setjmp(png_jmpbuf(p->png)))
        return -1;
    png_read_image(p->png, rows);
    return 0;
}

/* The version strings pdfTeX's --version prints. */
const char *ftpng_header_version(void)
{
    return PNG_LIBPNG_VER_STRING;
}

const char *ftpng_lib_version(void)
{
    return png_libpng_ver;
}
