#ifndef LEVIATHAN_GRAPHICS_H
#define LEVIATHAN_GRAPHICS_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/*
 * Leviathan Universal Color Representation
 */
struct color {
    uint8_t r;
    uint8_t g;
    uint8_t b;
    uint8_t a;
};

/* Common Color Helper Macros */
#define COLOR_RGB(r, g, b)       ((struct color){(r), (g), (b), 255})
#define COLOR_RGBA(r, g, b, a)    ((struct color){(r), (g), (b), (a)})

#define COLOR_BLACK              COLOR_RGB(0, 0, 0)
#define COLOR_WHITE              COLOR_RGB(255, 255, 255)
#define COLOR_RED                COLOR_RGB(255, 0, 0)
#define COLOR_GREEN              COLOR_RGB(0, 255, 0)
#define COLOR_BLUE               COLOR_RGB(0, 0, 255)
#define COLOR_CYAN               COLOR_RGB(0, 255, 255)
#define COLOR_MAGENTA            COLOR_RGB(255, 0, 255)
#define COLOR_YELLOW             COLOR_RGB(255, 255, 0)

/* Core Lifecycle */
int graphics_init(void);
void graphics_shutdown(void);

/* Display Information */
uint32_t graphics_width(void);
uint32_t graphics_height(void);
uint32_t graphics_pitch(void);
uint32_t graphics_bpp(void);

/* Primitive Drawing */
void graphics_clear(struct color c);
void graphics_put_pixel(int x, int y, struct color c);
struct color graphics_get_pixel(int x, int y);

/* Shapes */
void graphics_draw_line(int x0, int y0, int x1, int y1, struct color c);
void graphics_draw_rect(int x, int y, int w, int h, struct color c);
void graphics_fill_rect(int x, int y, int w, int h, struct color c);
void graphics_draw_circle(int cx, int cy, int radius, struct color c);
void graphics_fill_circle(int cx, int cy, int radius, struct color c);

/* Text Rendering */
void graphics_draw_char(int x, int y, char ch, struct color fg, struct color bg);
void graphics_draw_text(int x, int y, const char *str, struct color fg, struct color bg);

/* Double Buffering (Flicker-Free 60 FPS) */
void graphics_swap_buffers(void);
void graphics_copy_to_draw_buffer(const void *src, size_t size);
uint8_t *graphics_get_draw_buffer(void);

/* Linux Virtual Terminal Console Management (Silences cursor & console text) */
void graphics_silence_vt(bool enable);

#endif /* LEVIATHAN_GRAPHICS_H */
