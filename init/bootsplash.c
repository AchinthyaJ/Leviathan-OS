#include "graphics.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <unistd.h>
#include <math.h>
#include <time.h>
#include <fcntl.h>
#include <signal.h>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

#define NUM_STARS 75
#define NUM_RIPPLES 4

static volatile bool s_running = true;

static void handle_sig(int sig)
{
    (void)sig;
    s_running = false;
}

typedef struct {
    float x, y;
    float speed;
    float brightness;
    int size;
} Star;

static Star s_stars[NUM_STARS];

static void init_stars(uint32_t width, uint32_t height)
{
    srand(42);
    for (int i = 0; i < NUM_STARS; i++) {
        s_stars[i].x = (float)(rand() % (width ? width : 1920));
        s_stars[i].y = (float)(rand() % (height ? height : 1080));
        s_stars[i].speed = 0.5f + ((float)(rand() % 100) / 100.0f) * 1.5f;
        s_stars[i].brightness = (float)(rand() % 100) / 100.0f;
        s_stars[i].size = 1 + (rand() % 3);
    }
}

/* Smooth gradient background */
static void render_background(uint32_t width, uint32_t height)
{
    for (uint32_t y = 0; y < height; y++) {
        float t = (float)y / (float)height;
        /* Deep midnight ocean gradient: #030712 down to #0a192f */
        uint8_t r = (uint8_t)(3 + t * 7);
        uint8_t g = (uint8_t)(7 + t * 18);
        uint8_t b = (uint8_t)(18 + t * 30);
        struct color c = COLOR_RGB(r, g, b);
        graphics_draw_line(0, y, width - 1, y, c);
    }
}

/* Glowing cyber circle / logo */
static void render_pulsing_emblem(int cx, int cy, float time_sec)
{
    float pulse = (sinf(time_sec * 3.5f) + 1.0f) * 0.5f; // 0.0 to 1.0
    int base_radius = 48;
    int outer_radius = base_radius + (int)(pulse * 8.0f);

    /* Outer glow ring */
    for (int r = outer_radius; r >= base_radius; r--) {
        float alpha = (float)(r - base_radius) / 8.0f;
        uint8_t glow_g = (uint8_t)(180 + (1.0f - alpha) * 60);
        uint8_t glow_b = (uint8_t)(220 + (1.0f - alpha) * 35);
        struct color ring_col = COLOR_RGB(0, glow_g, glow_b);
        graphics_draw_circle(cx, cy, r, ring_col);
    }

    /* Core disk */
    graphics_fill_circle(cx, cy, base_radius - 2, COLOR_RGB(8, 20, 42));
    graphics_draw_circle(cx, cy, base_radius - 2, COLOR_RGB(97, 175, 239));

    /* Inner rotating radar or energy tick */
    float angle = time_sec * 4.0f;
    int tx = cx + (int)(cosf(angle) * (base_radius - 12));
    int ty = cy + (int)(sinf(angle) * (base_radius - 12));
    graphics_fill_circle(tx, ty, 5, COLOR_RGB(0, 229, 255));

    int tx2 = cx + (int)(cosf(angle + M_PI) * (base_radius - 12));
    int ty2 = cy + (int)(sinf(angle + M_PI) * (base_radius - 12));
    graphics_fill_circle(tx2, ty2, 4, COLOR_RGB(198, 120, 221));

    /* Center emblem character */
    graphics_fill_circle(cx, cy, 6, COLOR_RGB(255, 255, 255));
}

int main(int argc, char *argv[])
{
    signal(SIGTERM, handle_sig);
    signal(SIGINT, handle_sig);

    float max_seconds = 10.0f;
    if (argc > 1) {
        max_seconds = (float)atof(argv[1]);
    }

    if (graphics_init() != 0) {
        return 1;
    }

    uint32_t width = graphics_width();
    uint32_t height = graphics_height();

    init_stars(width, height);

    /* Allocate background cache buffer */
    size_t screen_bytes = graphics_pitch() * height;
    uint8_t *bg_cache = (uint8_t *)malloc(screen_bytes);
    if (bg_cache) {
        render_background(width, height);
        /* Copy to cache */
        uint8_t *db = graphics_get_draw_buffer();
        if (db) memcpy(bg_cache, db, screen_bytes);
    }

    int cx = (int)width / 2;
    int cy = (int)height / 2 - 40;

    struct timespec start_time, frame_start, frame_end;
    clock_gettime(CLOCK_MONOTONIC, &start_time);

    const long TARGET_FRAME_NS = 16666666; // 60 FPS

    while (s_running) {
        clock_gettime(CLOCK_MONOTONIC, &frame_start);

        float elapsed = (float)(frame_start.tv_sec - start_time.tv_sec) +
                        (float)(frame_start.tv_nsec - start_time.tv_nsec) / 1e9f;

        if (max_seconds > 0.0f && elapsed >= max_seconds) {
            break;
        }

        /* Fast background blit */
        if (bg_cache) {
            graphics_copy_to_draw_buffer(bg_cache, screen_bytes);
        } else {
            graphics_clear(COLOR_RGB(5, 12, 28));
        }

        /* Render stars/particles */
        for (int i = 0; i < NUM_STARS; i++) {
            s_stars[i].y -= s_stars[i].speed;
            if (s_stars[i].y < 0) {
                s_stars[i].y = (float)height;
                s_stars[i].x = (float)(rand() % width);
            }
            float flicker = (sinf(elapsed * 4.0f + s_stars[i].brightness * 10.0f) + 1.0f) * 0.5f;
            uint8_t bri = (uint8_t)(80 + flicker * 175);
            struct color star_col = COLOR_RGB(bri, (uint8_t)(bri * 0.9f), 255);
            graphics_fill_circle((int)s_stars[i].x, (int)s_stars[i].y, s_stars[i].size, star_col);
        }

        /* Center pulsing emblem */
        render_pulsing_emblem(cx, cy, elapsed);

        /* OS Title */
        const char *title = "L E V I A T H A N   O S";
        int title_len = (int)strlen(title);
        int title_x = cx - (title_len * 8) / 2;
        int title_y = cy + 75;
        graphics_draw_text(title_x, title_y, title, COLOR_RGB(255, 255, 255), COLOR_RGBA(0, 0, 0, 0));

        /* Progress track */
        int bar_w = 260;
        int bar_h = 4;
        int bar_x = cx - bar_w / 2;
        int bar_y = title_y + 35;
        graphics_fill_rect(bar_x, bar_y, bar_w, bar_h, COLOR_RGB(20, 32, 55));

        /* Moving glowing scan beam inside progress bar */
        float bar_progress = fmodf(elapsed * 0.8f, 1.0f);
        int beam_len = 70;
        int beam_pos = bar_x + (int)(bar_progress * (bar_w + beam_len)) - beam_len;
        int x_start = beam_pos < bar_x ? bar_x : beam_pos;
        int x_end = (beam_pos + beam_len) > (bar_x + bar_w) ? (bar_x + bar_w) : (beam_pos + beam_len);
        if (x_end > x_start) {
            graphics_fill_rect(x_start, bar_y, x_end - x_start, bar_h, COLOR_RGB(0, 229, 255));
        }

        /* Subtitle */
        const char *status_msg = "Starting Wayland Desktop Environment...";
        int status_len = (int)strlen(status_msg);
        int status_x = cx - (status_len * 8) / 2;
        graphics_draw_text(status_x, bar_y + 16, status_msg, COLOR_RGB(138, 175, 215), COLOR_RGBA(0, 0, 0, 0));

        graphics_swap_buffers();

        /* 60 FPS pacing */
        clock_gettime(CLOCK_MONOTONIC, &frame_end);
        long elapsed_ns = (frame_end.tv_sec - frame_start.tv_sec) * 1000000000L +
                          (frame_end.tv_nsec - frame_start.tv_nsec);
        long sleep_ns = TARGET_FRAME_NS - elapsed_ns;
        if (sleep_ns > 300000L) {
            struct timespec req = {0, sleep_ns};
            nanosleep(&req, NULL);
        }
    }

    if (bg_cache) free(bg_cache);
    graphics_shutdown();
    return 0;
}
