#include "graphics.h"
#include "font8x8.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <linux/fb.h>
#include <linux/kd.h>

/* Internal State */
static int s_fb_fd = -1;
static uint8_t *s_fb_mem = NULL;
static uint8_t *s_backbuffer = NULL;
static uint8_t *s_draw_buffer = NULL;
static struct fb_fix_screeninfo s_finfo;
static struct fb_var_screeninfo s_vinfo;
static uint32_t s_bytes_per_pixel = 0;
static bool s_initialized = false;

static inline uint32_t pack_color(struct color c)
{
    uint32_t r = (c.r >> (8 - s_vinfo.red.length)) << s_vinfo.red.offset;
    uint32_t g = (c.g >> (8 - s_vinfo.green.length)) << s_vinfo.green.offset;
    uint32_t b = (c.b >> (8 - s_vinfo.blue.length)) << s_vinfo.blue.offset;
    uint32_t a = (s_vinfo.transp.length > 0) ?
                 ((c.a >> (8 - s_vinfo.transp.length)) << s_vinfo.transp.offset) : 0;
    return r | g | b | a;
}

static inline struct color unpack_color(uint32_t p)
{
    struct color c;
    c.r = (uint8_t)(((p >> s_vinfo.red.offset) & ((1 << s_vinfo.red.length) - 1)) << (8 - s_vinfo.red.length));
    c.g = (uint8_t)(((p >> s_vinfo.green.offset) & ((1 << s_vinfo.green.length) - 1)) << (8 - s_vinfo.green.length));
    c.b = (uint8_t)(((p >> s_vinfo.blue.offset) & ((1 << s_vinfo.blue.length) - 1)) << (8 - s_vinfo.blue.length));
    c.a = (s_vinfo.transp.length > 0) ?
          (uint8_t)(((p >> s_vinfo.transp.offset) & ((1 << s_vinfo.transp.length) - 1)) << (8 - s_vinfo.transp.length)) : 255;
    return c;
}

void graphics_silence_vt(bool enable)
{
    int tty_fd = open("/dev/tty0", O_RDWR);
    if (tty_fd < 0) tty_fd = open("/dev/tty1", O_RDWR);
    if (tty_fd < 0) tty_fd = open("/dev/console", O_RDWR);
    if (tty_fd >= 0) {
        ioctl(tty_fd, KDSETMODE, enable ? KD_GRAPHICS : KD_TEXT);
        close(tty_fd);
    }
}

int graphics_init(void)
{
    if (s_initialized)
        return 0;

    const char *fb_path = "/dev/fb0";
    s_fb_fd = open(fb_path, O_RDWR);
    if (s_fb_fd < 0) {
        perror("graphics_init: open /dev/fb0 failed");
        return -1;
    }

    if (ioctl(s_fb_fd, FBIOGET_FSCREENINFO, &s_finfo) < 0) {
        perror("graphics_init: ioctl FBIOGET_FSCREENINFO failed");
        close(s_fb_fd);
        s_fb_fd = -1;
        return -1;
    }

    if (ioctl(s_fb_fd, FBIOGET_VSCREENINFO, &s_vinfo) < 0) {
        perror("graphics_init: ioctl FBIOGET_VSCREENINFO failed");
        close(s_fb_fd);
        s_fb_fd = -1;
        return -1;
    }

    s_bytes_per_pixel = s_vinfo.bits_per_pixel / 8;
    if (s_bytes_per_pixel != 4 && s_bytes_per_pixel != 2 && s_bytes_per_pixel != 3) {
        fprintf(stderr, "graphics_init: unsupported bits per pixel: %u\n", s_vinfo.bits_per_pixel);
        close(s_fb_fd);
        s_fb_fd = -1;
        return -1;
    }

    s_fb_mem = (uint8_t *)mmap(NULL, s_finfo.smem_len, PROT_READ | PROT_WRITE, MAP_SHARED, s_fb_fd, 0);
    if (s_fb_mem == MAP_FAILED) {
        perror("graphics_init: mmap failed");
        close(s_fb_fd);
        s_fb_fd = -1;
        s_fb_mem = NULL;
        return -1;
    }

    /* Allocate RAM back-buffer for tear-free 60 FPS double-buffering */
    s_backbuffer = (uint8_t *)malloc(s_finfo.smem_len);
    if (s_backbuffer) {
        s_draw_buffer = s_backbuffer;
        memset(s_backbuffer, 0, s_finfo.smem_len);
    } else {
        /* Fall back to direct framebuffer if RAM is tight */
        s_draw_buffer = s_fb_mem;
    }

    /* Silence Linux VT cursor and console text while in graphics */
    graphics_silence_vt(true);

    s_initialized = true;
    return 0;
}

void graphics_shutdown(void)
{
    if (!s_initialized)
        return;

    /* Restore Linux VT text mode so TTY console is normal */
    graphics_silence_vt(false);

    if (s_backbuffer) {
        free(s_backbuffer);
        s_backbuffer = NULL;
    }
    s_draw_buffer = NULL;

    if (s_fb_mem && s_fb_mem != MAP_FAILED) {
        munmap(s_fb_mem, s_finfo.smem_len);
        s_fb_mem = NULL;
    }

    if (s_fb_fd >= 0) {
        close(s_fb_fd);
        s_fb_fd = -1;
    }

    s_initialized = false;
}

void graphics_swap_buffers(void)
{
    if (s_initialized && s_backbuffer && s_fb_mem) {
        memcpy(s_fb_mem, s_backbuffer, s_finfo.smem_len);
    }
}

void graphics_copy_to_draw_buffer(const void *src, size_t size)
{
    if (s_draw_buffer && src) {
        size_t copy_len = (size <= s_finfo.smem_len) ? size : s_finfo.smem_len;
        memcpy(s_draw_buffer, src, copy_len);
    }
}

uint8_t *graphics_get_draw_buffer(void)
{
    return s_draw_buffer;
}

uint32_t graphics_width(void)  { return s_vinfo.xres; }
uint32_t graphics_height(void) { return s_vinfo.yres; }
uint32_t graphics_pitch(void)  { return s_finfo.line_length; }
uint32_t graphics_bpp(void)    { return s_vinfo.bits_per_pixel; }

void graphics_put_pixel(int x, int y, struct color c)
{
    if (!s_initialized || !s_draw_buffer)
        return;

    /* Strict bounds check */
    if (x < 0 || (uint32_t)x >= s_vinfo.xres || y < 0 || (uint32_t)y >= s_vinfo.yres)
        return;

    size_t offset = (size_t)y * s_finfo.line_length + (size_t)x * s_bytes_per_pixel;
    if (offset + s_bytes_per_pixel > s_finfo.smem_len)
        return;

    uint32_t pval = pack_color(c);

    if (s_bytes_per_pixel == 4) {
        *(volatile uint32_t *)(s_draw_buffer + offset) = pval;
    } else if (s_bytes_per_pixel == 2) {
        *(volatile uint16_t *)(s_draw_buffer + offset) = (uint16_t)pval;
    } else if (s_bytes_per_pixel == 3) {
        s_draw_buffer[offset + 0] = c.b;
        s_draw_buffer[offset + 1] = c.g;
        s_draw_buffer[offset + 2] = c.r;
    }
}

struct color graphics_get_pixel(int x, int y)
{
    if (!s_initialized || !s_draw_buffer)
        return COLOR_BLACK;

    if (x < 0 || (uint32_t)x >= s_vinfo.xres || y < 0 || (uint32_t)y >= s_vinfo.yres)
        return COLOR_BLACK;

    size_t offset = (size_t)y * s_finfo.line_length + (size_t)x * s_bytes_per_pixel;
    if (offset + s_bytes_per_pixel > s_finfo.smem_len)
        return COLOR_BLACK;

    uint32_t pval = 0;
    if (s_bytes_per_pixel == 4) {
        pval = *(volatile uint32_t *)(s_draw_buffer + offset);
    } else if (s_bytes_per_pixel == 2) {
        pval = *(volatile uint16_t *)(s_draw_buffer + offset);
    } else if (s_bytes_per_pixel == 3) {
        struct color c;
        c.b = s_draw_buffer[offset + 0];
        c.g = s_draw_buffer[offset + 1];
        c.r = s_draw_buffer[offset + 2];
        c.a = 255;
        return c;
    }

    return unpack_color(pval);
}

void graphics_clear(struct color c)
{
    if (!s_initialized || !s_draw_buffer)
        return;

    uint32_t pval = pack_color(c);

    for (uint32_t y = 0; y < s_vinfo.yres; y++) {
        uint8_t *row = s_draw_buffer + (size_t)y * s_finfo.line_length;
        if (s_bytes_per_pixel == 4) {
            uint32_t *dst = (uint32_t *)row;
            for (uint32_t x = 0; x < s_vinfo.xres; x++) {
                dst[x] = pval;
            }
        } else if (s_bytes_per_pixel == 2) {
            uint16_t *dst = (uint16_t *)row;
            for (uint32_t x = 0; x < s_vinfo.xres; x++) {
                dst[x] = (uint16_t)pval;
            }
        } else {
            for (uint32_t x = 0; x < s_vinfo.xres; x++) {
                graphics_put_pixel((int)x, (int)y, c);
            }
        }
    }
}

void graphics_fill_rect(int x, int y, int w, int h, struct color c)
{
    if (!s_initialized || !s_draw_buffer || w <= 0 || h <= 0)
        return;

    int x0 = x;
    int y0 = y;
    int x1 = x + w;
    int y1 = y + h;

    /* Clip rectangle */
    if (x0 < 0) x0 = 0;
    if (y0 < 0) y0 = 0;
    if (x1 > (int)s_vinfo.xres) x1 = (int)s_vinfo.xres;
    if (y1 > (int)s_vinfo.yres) y1 = (int)s_vinfo.yres;

    if (x0 >= x1 || y0 >= y1)
        return;

    uint32_t pval = pack_color(c);

    for (int cy = y0; cy < y1; cy++) {
        uint8_t *row = s_draw_buffer + (size_t)cy * s_finfo.line_length;
        if (s_bytes_per_pixel == 4) {
            uint32_t *dst = (uint32_t *)row;
            for (int cx = x0; cx < x1; cx++) {
                dst[cx] = pval;
            }
        } else if (s_bytes_per_pixel == 2) {
            uint16_t *dst = (uint16_t *)row;
            for (int cx = x0; cx < x1; cx++) {
                dst[cx] = (uint16_t)pval;
            }
        } else {
            for (int cx = x0; cx < x1; cx++) {
                graphics_put_pixel(cx, cy, c);
            }
        }
    }
}

void graphics_draw_rect(int x, int y, int w, int h, struct color c)
{
    if (w <= 0 || h <= 0)
        return;

    graphics_draw_line(x, y, x + w - 1, y, c);
    graphics_draw_line(x, y + h - 1, x + w - 1, y + h - 1, c);
    graphics_draw_line(x, y, x, y + h - 1, c);
    graphics_draw_line(x + w - 1, y, x + w - 1, y + h - 1, c);
}

void graphics_draw_line(int x0, int y0, int x1, int y1, struct color c)
{
    int dx = abs(x1 - x0);
    int sx = (x0 < x1) ? 1 : -1;
    int dy = -abs(y1 - y0);
    int sy = (y0 < y1) ? 1 : -1;
    int err = dx + dy;

    while (1) {
        graphics_put_pixel(x0, y0, c);
        if (x0 == x1 && y0 == y1)
            break;
        int e2 = 2 * err;
        if (e2 >= dy) {
            err += dy;
            x0 += sx;
        }
        if (e2 <= dx) {
            err += dx;
            y0 += sy;
        }
    }
}

void graphics_draw_circle(int cx, int cy, int radius, struct color c)
{
    if (radius < 0)
        return;

    int x = radius;
    int y = 0;
    int err = 0;

    while (x >= y) {
        graphics_put_pixel(cx + x, cy + y, c);
        graphics_put_pixel(cx + y, cy + x, c);
        graphics_put_pixel(cx - y, cy + x, c);
        graphics_put_pixel(cx - x, cy + y, c);
        graphics_put_pixel(cx - x, cy - y, c);
        graphics_put_pixel(cx - y, cy - x, c);
        graphics_put_pixel(cx + y, cy - x, c);
        graphics_put_pixel(cx + x, cy - y, c);

        if (err <= 0) {
            y += 1;
            err += 2 * y + 1;
        }
        if (err > 0) {
            x -= 1;
            err -= 2 * x + 1;
        }
    }
}

void graphics_fill_circle(int cx, int cy, int radius, struct color c)
{
    if (radius < 0)
        return;

    int r2 = radius * radius;
    for (int dy = -radius; dy <= radius; dy++) {
        int dx_max = 0;
        while ((dx_max + 1) * (dx_max + 1) + dy * dy <= r2) {
            dx_max++;
        }
        graphics_draw_line(cx - dx_max, cy + dy, cx + dx_max, cy + dy, c);
    }
}

void graphics_draw_char(int x, int y, char ch, struct color fg, struct color bg)
{
    unsigned char uch = (unsigned char)ch;
    if (uch < 32 || uch > 127)
        uch = '?';

    const uint8_t *bitmap = font8x8_basic[(int)uch - 32];

    for (int row = 0; row < 8; row++) {
        uint8_t byte = bitmap[row];
        for (int col = 0; col < 8; col++) {
            if (byte & (0x80 >> col)) {
                graphics_put_pixel(x + col, y + row, fg);
            } else if (bg.a > 0) {
                graphics_put_pixel(x + col, y + row, bg);
            }
        }
    }
}

void graphics_draw_text(int x, int y, const char *str, struct color fg, struct color bg)
{
    if (!str)
        return;

    int cur_x = x;
    int cur_y = y;

    while (*str) {
        if (*str == '\n') {
            cur_x = x;
            cur_y += 10;
        } else if (*str == '\t') {
            cur_x += 32;
        } else {
            graphics_draw_char(cur_x, cur_y, *str, fg, bg);
            cur_x += 8;
        }
        str++;
    }
}
