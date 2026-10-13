#include "graphics.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <unistd.h>
#include <math.h>
#include <time.h>
#include <termios.h>
#include <fcntl.h>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

#define NUM_BODY_SEGMENTS 115
#define NUM_BUBBLES 45
#define NUM_MOTES 60

/* Pre-rendered background cache */
static uint8_t *s_bg_cache = NULL;
static size_t s_bg_cache_size = 0;

/* Color blending helper */
static inline struct color blend_color(struct color base, struct color add, float alpha)
{
    if (alpha <= 0.0f) return base;
    if (alpha >= 1.0f) return add;
    struct color res;
    res.r = (uint8_t)(base.r * (1.0f - alpha) + add.r * alpha);
    res.g = (uint8_t)(base.g * (1.0f - alpha) + add.g * alpha);
    res.b = (uint8_t)(base.b * (1.0f - alpha) + add.b * alpha);
    res.a = 255;
    return res;
}

/* Bubble Particle Structure */
typedef struct {
    float x, y;
    float speed_y;
    float wobble_speed;
    float wobble_amp;
    int radius;
    float seed;
} Bubble;

/* Marine Snow Mote Structure */
typedef struct {
    float x, y;
    float speed_y;
    float speed_x;
    int brightness;
} Mote;

static Bubble s_bubbles[NUM_BUBBLES];
static Mote s_motes[NUM_MOTES];

static void init_particles(uint32_t width, uint32_t height)
{
    srand(1337);
    for (int i = 0; i < NUM_BUBBLES; i++) {
        s_bubbles[i].x = (float)(rand() % width);
        s_bubbles[i].y = (float)(rand() % height);
        s_bubbles[i].speed_y = 1.2f + ((float)(rand() % 100) / 100.0f) * 2.2f;
        s_bubbles[i].wobble_speed = 2.0f + ((float)(rand() % 100) / 100.0f) * 3.0f;
        s_bubbles[i].wobble_amp = 1.0f + ((float)(rand() % 100) / 100.0f) * 2.5f;
        s_bubbles[i].radius = 2 + (rand() % 6);
        s_bubbles[i].seed = (float)(rand() % 100);
    }

    for (int i = 0; i < NUM_MOTES; i++) {
        s_motes[i].x = (float)(rand() % width);
        s_motes[i].y = (float)(rand() % height);
        s_motes[i].speed_y = 0.3f + ((float)(rand() % 100) / 100.0f) * 0.5f;
        s_motes[i].speed_x = -0.2f + ((float)(rand() % 100) / 100.0f) * 0.4f;
        s_motes[i].brightness = 70 + rand() % 150;
    }
}

static void update_particles(uint32_t width, uint32_t height, float time_sec)
{
    for (int i = 0; i < NUM_BUBBLES; i++) {
        s_bubbles[i].y -= s_bubbles[i].speed_y;
        s_bubbles[i].x += sinf(time_sec * s_bubbles[i].wobble_speed + s_bubbles[i].seed) * 0.6f;

        if (s_bubbles[i].y < 45.0f) {
            s_bubbles[i].y = (float)(height - 40 - (rand() % 60));
            s_bubbles[i].x = (float)(rand() % width);
        }
    }

    for (int i = 0; i < NUM_MOTES; i++) {
        s_motes[i].y += s_motes[i].speed_y;
        s_motes[i].x += s_motes[i].speed_x + sinf(time_sec * 0.5f + (float)i) * 0.2f;

        if (s_motes[i].y >= (float)(height - 10)) {
            s_motes[i].y = 50.0f;
            s_motes[i].x = (float)(rand() % width);
        }
        if (s_motes[i].x < 0.0f) s_motes[i].x += (float)width;
        if (s_motes[i].x >= (float)width) s_motes[i].x -= (float)width;
    }
}

/* Pre-render the static deep ocean background into s_bg_cache */
static void init_background_cache(uint32_t width, uint32_t height)
{
    s_bg_cache_size = graphics_pitch() * height;
    s_bg_cache = (uint8_t *)malloc(s_bg_cache_size);
    if (!s_bg_cache) {
        fprintf(stderr, "leviathan-art: failed to allocate background cache\n");
        return;
    }

    printf("leviathan-art: Pre-rendering deep ocean background cache...\n");

    /* 1. Vertical ocean depth gradient */
    for (uint32_t y = 0; y < height; y++) {
        float ty = (float)y / (float)height;
        uint8_t r = (uint8_t)(10 * (1.0f - ty) * (1.0f - ty) + 4 * (1.0f - ty) * ty * 2 + 1 * ty);
        uint8_t g = (uint8_t)(45 * (1.0f - ty) + 12 * ty);
        uint8_t b = (uint8_t)(80 * (1.0f - ty) + 20 * ty);

        struct color row_col = COLOR_RGB(r, g, b);
        for (uint32_t x = 0; x < width; x++) {
            graphics_put_pixel((int)x, (int)y, row_col);
        }
    }

    /* 2. Ethereal light shafts / sunbeams */
    struct {
        float base_x;
        float width_top;
        float slant;
        float base_int;
    } rays[] = {
        {160.0f, 65.0f,  0.34f, 0.12f},
        {360.0f, 90.0f,  0.40f, 0.18f},
        {570.0f, 115.0f, 0.36f, 0.15f},
        {810.0f, 80.0f,  0.43f, 0.12f},
        {1050.0f, 95.0f, 0.38f, 0.14f},
    };

    for (size_t i = 0; i < sizeof(rays) / sizeof(rays[0]); i++) {
        float start_x = rays[i].base_x;
        float w_top   = rays[i].width_top;
        float slant   = rays[i].slant;
        float intensity = rays[i].base_int;

        for (uint32_t y = 0; y < height * 0.72f; y++) {
            float y_ratio = (float)y / (float)(height * 0.72f);
            float curr_center_x = start_x + (float)y * slant;
            float curr_width = w_top * (1.0f + y_ratio * 1.6f);
            float beam_falloff = (1.0f - y_ratio) * intensity;

            int x_min = (int)(curr_center_x - curr_width * 0.5f);
            int x_max = (int)(curr_center_x + curr_width * 0.5f);

            for (int x = x_min; x <= x_max; x++) {
                if (x < 0 || (uint32_t)x >= width) continue;
                float dist = fabsf((float)x - curr_center_x) / (curr_width * 0.5f);
                if (dist > 1.0f) continue;

                float alpha = (1.0f - dist * dist) * beam_falloff;
                struct color cur = graphics_get_pixel(x, y);
                struct color ray_c = COLOR_RGB(80, 190, 230);
                graphics_put_pixel(x, y, blend_color(cur, ray_c, alpha));
            }
        }
    }

    /* 3. Seafloor mountain ridges & trenches */
    for (int x = 0; x < (int)width; x++) {
        float fx = (float)x * 0.008f;
        int hill_y = (int)(height - 110 + sinf(fx * 3.1f) * 35.0f + cosf(fx * 1.7f) * 25.0f);
        for (int y = hill_y; y < (int)height; y++) {
            struct color cur = graphics_get_pixel(x, y);
            struct color rock = COLOR_RGB(3, 14, 26);
            graphics_put_pixel(x, y, blend_color(cur, rock, 0.85f));
        }
    }

    for (int x = 0; x < (int)width; x++) {
        float fx = (float)x * 0.015f;
        int rock_y = (int)(height - 65 + sinf(fx * 4.2f) * 22.0f + cosf(fx * 7.5f) * 12.0f);
        for (int y = rock_y; y < (int)height; y++) {
            graphics_put_pixel(x, y, COLOR_RGB(2, 8, 16));
        }
    }

    /* Copy rendered background into s_bg_cache */
    uint8_t *draw_buf = graphics_get_draw_buffer();
    if (draw_buf) {
        memcpy(s_bg_cache, draw_buf, s_bg_cache_size);
    }
}

/* 3. The Animated Swimming Leviathan at 60 FPS */
typedef struct {
    float x, y;
    float angle;
    float radius;
} BodySegment;

static void render_animated_leviathan(uint32_t width, uint32_t height, float time_sec)
{
    (void)width; (void)height;

    /*
     * Organic Swimming Trajectory across the deep ocean:
     * Head traces a graceful, undulating patrol path
     */
    float swim_speed = 0.55f;
    float t_swim = time_sec * swim_speed;

    /* Patrol center and heading */
    float head_x = 640.0f + 350.0f * sinf(t_swim * 1.1f);
    float head_y = 380.0f + 120.0f * sinf(t_swim * 1.7f);

    /* Velocity derivatives for heading angle */
    float vx = 350.0f * 1.1f * cosf(t_swim * 1.1f);
    float vy = 120.0f * 1.7f * cosf(t_swim * 1.7f);
    float head_angle = atan2f(vy, vx);

    /* Generate body segments using propagating sinusoidal spinal wave */
    BodySegment segs[NUM_BODY_SEGMENTS];
    segs[0].x = head_x;
    segs[0].y = head_y;
    segs[0].angle = head_angle;
    segs[0].radius = 28.0f;

    float segment_dist = 8.5f;

    for (int i = 1; i < NUM_BODY_SEGMENTS; i++) {
        float progress = (float)i / (float)NUM_BODY_SEGMENTS;

        /* Traveling undulation wave: propagates backwards from head to tail */
        float wave_freq = 0.12f;
        float wave_speed = 4.8f;
        float wave_amp = 0.035f + 0.38f * progress;

        float wave_offset = wave_amp * sinf((float)i * wave_freq - time_sec * wave_speed);
        float prev_angle = segs[i - 1].angle;
        float cur_angle = prev_angle + wave_offset;

        segs[i].x = segs[i - 1].x - cosf(cur_angle) * segment_dist;
        segs[i].y = segs[i - 1].y - sinf(cur_angle) * segment_dist;
        segs[i].angle = cur_angle;

        /* Muscular body envelope */
        float body_envelope = sinf(progress * (float)M_PI);
        if (progress < 0.15f) {
            segs[i].radius = 28.0f + (progress / 0.15f) * 10.0f;
        } else {
            segs[i].radius = 8.0f + 30.0f * powf(body_envelope, 0.7f);
        }
    }

    /* A. Draw Dorsal Spines & Flowing Fins (drawn behind body) */
    for (int i = 12; i < NUM_BODY_SEGMENTS - 15; i += 3) {
        float r = segs[i].radius;
        float a = segs[i].angle;

        /* Normal perpendicular to spine */
        float nx = -sinf(a);
        float ny =  cosf(a);

        /* Ensure spines orient towards dorsal side */
        if (ny > 0) { nx = -nx; ny = -ny; }

        float spine_ripple = sinf((float)i * 0.2f - time_sec * 5.0f);
        float spine_len = r * 1.7f + 16.0f * spine_ripple;
        if (spine_len < r + 4.0f) spine_len = r + 4.0f;

        int sx0 = (int)(segs[i].x + nx * r);
        int sy0 = (int)(segs[i].y + ny * r);
        int sx1 = (int)(segs[i].x + nx * spine_len);
        int sy1 = (int)(segs[i].y + ny * spine_len);

        /* Webbed fin connection */
        if (i > 15) {
            int prev_sx1 = (int)(segs[i - 3].x + nx * (spine_len * 0.8f));
            int prev_sy1 = (int)(segs[i - 3].y + ny * (spine_len * 0.8f));
            graphics_draw_line(sx1, sy1, prev_sx1, prev_sy1, COLOR_RGB(0, 180, 215));
        }

        graphics_draw_line(sx0, sy0, sx1, sy1, COLOR_RGB(10, 95, 125));
        graphics_fill_circle(sx1, sy1, 2, COLOR_RGB(90, 255, 240));
    }

    /* B. Caudal Tail Fin (flapping with tail motion) */
    {
        int last = NUM_BODY_SEGMENTS - 1;
        float tx = segs[last].x;
        float ty = segs[last].y;
        float ta = segs[last].angle;

        float tail_fin_spread = 45.0f;
        for (int fan = -4; fan <= 4; fan++) {
            float fan_a = ta + (float)fan * (tail_fin_spread * (float)M_PI / 180.0f) / 4.0f + (float)M_PI;
            float fin_len = 80.0f - fabsf((float)fan) * 8.0f;

            int fx = (int)(tx + cosf(fan_a) * fin_len);
            int fy = (int)(ty + sinf(fan_a) * fin_len);

            graphics_draw_line((int)tx, (int)ty, fx, fy, COLOR_RGB(0, 175, 220));
            graphics_fill_circle(fx, fy, 2, COLOR_RGB(130, 255, 245));
        }
    }

    /* C. Serpentine Armored Body Segments (tail to head) */
    for (int i = NUM_BODY_SEGMENTS - 1; i >= 0; i--) {
        int cx = (int)segs[i].x;
        int cy = (int)segs[i].y;
        int cr = (int)segs[i].radius;

        /* Outer armored midnight-teal scale */
        graphics_fill_circle(cx, cy, cr, COLOR_RGB(6, 32, 44));

        /* Emerald scute highlight */
        if (cr > 4) {
            graphics_fill_circle(cx, cy, cr - 4, COLOR_RGB(12, 68, 76));
        }

        /* Ventral pale seafoam plates */
        if (cr > 8) {
            float a = segs[i].angle;
            int under_x = cx + (int)(sinf(a) * (cr * 0.25f));
            int under_y = cy - (int)(cosf(a) * (cr * 0.25f));
            graphics_fill_circle(under_x, under_y, cr / 2, COLOR_RGB(24, 110, 105));
        }
    }

    /* D. Bioluminescent Runes along the Lateral Line */
    for (int i = 8; i < NUM_BODY_SEGMENTS - 8; i += 3) {
        int bx = (int)segs[i].x;
        int by = (int)segs[i].y;

        float glow_phase = sinf(time_sec * 3.0f + (float)i * 0.12f);
        uint8_t glow_g = (uint8_t)(210 + 45 * glow_phase);
        uint8_t glow_b = (uint8_t)(190 + 65 * glow_phase);

        graphics_fill_circle(bx, by, 3, COLOR_RGB(0, glow_g, glow_b));
        graphics_fill_circle(bx, by, 1, COLOR_RGB(255, 255, 255));

        if (i > 8) {
            int pbx = (int)segs[i - 3].x;
            int pby = (int)segs[i - 3].y;
            graphics_draw_line(pbx, pby, bx, by, COLOR_RGB(0, 190, 200));
        }
    }

    /* E. Majestic Sea Dragon Head */
    {
        float hx = segs[0].x;
        float hy = segs[0].y;
        float ha = segs[0].angle;

        float forward_x = cosf(ha);
        float forward_y = sinf(ha);
        float right_x = -sinf(ha);
        float right_y =  cosf(ha);

        /* Skull base */
        graphics_fill_circle((int)hx, (int)hy, 30, COLOR_RGB(8, 38, 50));
        graphics_fill_circle((int)hx, (int)hy, 25, COLOR_RGB(14, 75, 84));

        /* Elongated draconic snout */
        float snout_dist = 42.0f;
        float snout_x = hx + forward_x * snout_dist;
        float snout_y = hy + forward_y * snout_dist;

        graphics_fill_circle((int)(hx + forward_x * 20.0f), (int)(hy + forward_y * 20.0f), 20, COLOR_RGB(10, 52, 65));
        graphics_fill_circle((int)snout_x, (int)snout_y, 14, COLOR_RGB(12, 65, 78));

        /* Parted lower jaw */
        float jaw_x = hx + forward_x * 32.0f + right_x * 12.0f;
        float jaw_y = hy + forward_y * 32.0f + right_y * 12.0f;
        graphics_fill_circle((int)jaw_x, (int)jaw_y, 11, COLOR_RGB(8, 42, 54));

        /* Sharp glowing fangs */
        for (int f = 0; f < 3; f++) {
            float fx0 = snout_x - forward_x * (8.0f + f * 10.0f);
            float fy0 = snout_y - forward_y * (8.0f + f * 10.0f);
            float fx1 = fx0 + right_x * 9.0f;
            float fy1 = fy0 + right_y * 9.0f;
            graphics_draw_line((int)fx0, (int)fy0, (int)fx1, (int)fy1, COLOR_RGB(240, 255, 255));
        }

        /* Glowing Amber Eye (with occasional blinking) */
        float eye_x = hx + forward_x * 12.0f - right_x * 12.0f;
        float eye_y = hy + forward_y * 12.0f - right_y * 12.0f;

        float blink_timer = fmodf(time_sec, 4.0f);
        bool blinking = (blink_timer > 3.85f);

        if (blinking) {
            graphics_draw_line((int)(eye_x - forward_x * 6), (int)(eye_y - forward_y * 6),
                               (int)(eye_x + forward_x * 6), (int)(eye_y + forward_y * 6),
                               COLOR_RGB(0, 180, 160));
        } else {
            graphics_fill_circle((int)eye_x, (int)eye_y, 9, COLOR_RGB(0, 180, 160));
            graphics_fill_circle((int)eye_x, (int)eye_y, 6, COLOR_RGB(255, 185, 0));
            graphics_fill_circle((int)eye_x, (int)eye_y, 4, COLOR_RGB(255, 225, 60));
            graphics_draw_line((int)(eye_x - right_x * 5), (int)(eye_y - right_y * 5),
                               (int)(eye_x + right_x * 5), (int)(eye_y + right_y * 5),
                               COLOR_RGB(10, 10, 10));
        }

        /* Aquatic horns swept backward along the neck */
        for (int h = 0; h < 2; h++) {
            float horn_side = (h == 0) ? -1.0f : 1.0f;
            float h_len = 65.0f;
            int px = (int)(hx - right_x * (horn_side * 10.0f));
            int py = (int)(hy - right_y * (horn_side * 10.0f));

            for (int seg = 1; seg <= 12; seg++) {
                float st = (float)seg / 12.0f;
                float cur_hx = hx - forward_x * (st * h_len) + right_x * (horn_side * (10.0f + st * 18.0f)) - right_y * (st * st * 14.0f);
                float cur_hy = hy - forward_y * (st * h_len) + right_y * (horn_side * (10.0f + st * 18.0f)) + right_x * (st * st * 14.0f);

                graphics_draw_line(px, py, (int)cur_hx, (int)cur_hy, COLOR_RGB(18, 90 + seg * 9, 110 + seg * 10));
                if (seg == 12) {
                    graphics_fill_circle((int)cur_hx, (int)cur_hy, 3, COLOR_RGB(0, 255, 230));
                }
                px = (int)cur_hx;
                py = (int)cur_hy;
            }
        }

        /* Bioluminescent whiskers flowing behind snout */
        for (int w = 0; w < 2; w++) {
            float w_side = (w == 0) ? -1.0f : 1.0f;
            int prev_wx = (int)snout_x;
            int prev_wy = (int)snout_y;

            for (int seg = 1; seg <= 15; seg++) {
                float st = (float)seg / 15.0f;
                float drift = sinf(time_sec * 6.0f + st * 3.0f + (float)w) * 12.0f;
                float cur_wx = snout_x - forward_x * (st * 70.0f) + right_x * (w_side * (8.0f + st * 15.0f) + drift);
                float cur_wy = snout_y - forward_y * (st * 70.0f) + right_y * (w_side * (8.0f + st * 15.0f) + drift);

                graphics_draw_line(prev_wx, prev_wy, (int)cur_wx, (int)cur_wy, COLOR_RGB(0, 210, 200));
                if (seg == 15) {
                    graphics_fill_circle((int)cur_wx, (int)cur_wy, 3, COLOR_RGB(80, 255, 240));
                    graphics_fill_circle((int)cur_wx, (int)cur_wy, 1, COLOR_RGB(255, 255, 255));
                }
                prev_wx = (int)cur_wx;
                prev_wy = (int)cur_wy;
            }
        }
    }
}

/* 4. Render Floating Particles & Bubbles */
static void render_particles(void)
{
    for (int i = 0; i < NUM_BUBBLES; i++) {
        int bx = (int)s_bubbles[i].x;
        int by = (int)s_bubbles[i].y;
        int br = s_bubbles[i].radius;

        graphics_draw_circle(bx, by, br, COLOR_RGB(120, 220, 240));
        graphics_put_pixel(bx - br / 3, by - br / 3, COLOR_RGB(240, 255, 255));
    }

    for (int i = 0; i < NUM_MOTES; i++) {
        int px = (int)s_motes[i].x;
        int py = (int)s_motes[i].y;
        int b = s_motes[i].brightness;
        struct color c = COLOR_RGB(b / 2, b, b);
        graphics_put_pixel(px, py, c);
        if (b > 180) {
            graphics_put_pixel(px + 1, py, c);
        }
    }
}

/* 5. Sleek Real-Time HUD Overlay with Live FPS Counter */
static void render_hud(uint32_t width, uint32_t height, float fps, uint64_t frame_count)
{
    /* Top title bar */
    graphics_fill_rect(20, 12, width - 40, 32, COLOR_RGBA(5, 18, 30, 200));
    graphics_draw_rect(20, 12, width - 40, 32, COLOR_RGB(0, 180, 210));

    const char *title = "L E V I A T H A N   O S   -   6 0   F P S   D E E P   O C E A N   S C E N E R Y";
    int text_x = (width - (int)strlen(title) * 8) / 2;
    graphics_draw_text(text_x, 23, title, COLOR_RGB(0, 255, 230), COLOR_RGBA(0, 0, 0, 0));

    /* Bottom-left telemetry card */
    int card_w = 430;
    int card_h = 76;
    int card_x = 25;
    int card_y = height - card_h - 18;

    graphics_fill_rect(card_x, card_y, card_w, card_h, COLOR_RGBA(3, 14, 25, 220));
    graphics_draw_rect(card_x, card_y, card_w, card_h, COLOR_RGB(0, 160, 190));

    char buf[128];
    snprintf(buf, sizeof(buf), "PERFORMANCE: %.1f FPS | FRAME: %llu (60Hz Double-Buffered)",
             (double)fps, (unsigned long long)frame_count);
    graphics_draw_text(card_x + 12, card_y + 10, buf,
                       COLOR_RGB(80, 255, 140), COLOR_RGBA(0, 0, 0, 0));

    snprintf(buf, sizeof(buf), "HARDWARE:    SimpleDRM Framebuffer (%u x %u @ %u bpp)",
             graphics_width(), graphics_height(), graphics_bpp());
    graphics_draw_text(card_x + 12, card_y + 26, buf,
                       COLOR_RGB(140, 210, 230), COLOR_RGBA(0, 0, 0, 0));

    graphics_draw_text(card_x + 12, card_y + 42, "CONSOLE:     VT Text Muted (KD_GRAPHICS Active)",
                       COLOR_RGB(140, 210, 230), COLOR_RGBA(0, 0, 0, 0));

    graphics_draw_text(card_x + 12, card_y + 58, "PHYSICS:     Kinematic Serpentine Spine Simulation",
                       COLOR_RGB(0, 240, 220), COLOR_RGBA(0, 0, 0, 0));

    /* Bottom-right interactive exit prompt */
    const char *exit_msg = "Press [Q], [ESC], or [ENTER] to exit to shell";
    int exit_x = width - (int)strlen(exit_msg) * 8 - 35;
    graphics_fill_rect(exit_x - 10, height - 36, (int)strlen(exit_msg) * 8 + 20, 22, COLOR_RGBA(3, 14, 25, 220));
    graphics_draw_rect(exit_x - 10, height - 36, (int)strlen(exit_msg) * 8 + 20, 22, COLOR_RGB(0, 140, 180));
    graphics_draw_text(exit_x, height - 29, exit_msg, COLOR_RGB(180, 230, 245), COLOR_RGBA(0, 0, 0, 0));
}

int main(int argc, char *argv[])
{
    int max_frames = 0; /* 0 = infinite interactive loop */
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--frames") == 0 && i + 1 < argc) {
            max_frames = atoi(argv[++i]);
        } else if (strcmp(argv[i], "--timeout") == 0 && i + 1 < argc) {
            max_frames = atoi(argv[++i]) * 60;
        }
    }

    /* Initialize Leviathan double-buffered graphics (sets KD_GRAPHICS to mute TTY text) */
    if (graphics_init() != 0) {
        fprintf(stderr, "leviathan-art: failed to initialize graphics\n");
        return 1;
    }

    uint32_t width = graphics_width();
    uint32_t height = graphics_height();

    /* Pre-render static background into cache once for maximum performance */
    init_background_cache(width, height);
    init_particles(width, height);

    /* Setup non-blocking terminal input */
    struct termios orig_term, raw_term;
    bool term_set = false;
    if (isatty(STDIN_FILENO)) {
        if (tcgetattr(STDIN_FILENO, &orig_term) == 0) {
            raw_term = orig_term;
            raw_term.c_lflag &= ~(ICANON | ECHO);
            raw_term.c_cc[VMIN] = 0;
            raw_term.c_cc[VTIME] = 0;
            tcsetattr(STDIN_FILENO, TCSANOW, &raw_term);
            term_set = true;
        }
    }

    printf("leviathan-art: Starting 60 FPS swimming scenery on %ux%u...\n", width, height);

    struct timespec start_time, frame_start, frame_end;
    clock_gettime(CLOCK_MONOTONIC, &start_time);

    const long TARGET_FRAME_NS = 16666666; // 16.666 ms = 60 FPS
    uint64_t frame_count = 0;
    float current_fps = 60.0f;
    struct timespec last_fps_time = start_time;
    uint64_t last_fps_frame = 0;

    while (1) {
        clock_gettime(CLOCK_MONOTONIC, &frame_start);

        float time_sec = (float)(frame_start.tv_sec - start_time.tv_sec) +
                         (float)(frame_start.tv_nsec - start_time.tv_nsec) / 1000000000.0f;

        /* Update simulation */
        update_particles(width, height, time_sec);

        /* Fast-blit pre-rendered deep ocean background into backbuffer (~0.3 ms) */
        graphics_copy_to_draw_buffer(s_bg_cache, s_bg_cache_size);

        /* Render dynamic swimming Leviathan & particles */
        render_animated_leviathan(width, height, time_sec);
        render_particles();
        render_hud(width, height, current_fps, frame_count);

        /* Push complete frame to framebuffer in single memory burst (flicker-free) */
        graphics_swap_buffers();

        frame_count++;

        /* Compute measured FPS once per half-second */
        double elapsed_fps = (double)(frame_start.tv_sec - last_fps_time.tv_sec) +
                             (double)(frame_start.tv_nsec - last_fps_time.tv_nsec) / 1e9;
        if (elapsed_fps >= 0.5) {
            current_fps = (float)((frame_count - last_fps_frame) / elapsed_fps);
            last_fps_time = frame_start;
            last_fps_frame = frame_count;
        }

        /* Check for exit request */
        if (max_frames > 0 && (int)frame_count >= max_frames) {
            break;
        }

        /* Check keyboard input */
        if (term_set) {
            char ch = 0;
            if (read(STDIN_FILENO, &ch, 1) > 0) {
                if (ch == 'q' || ch == 'Q' || ch == 27 || ch == '\n' || ch == 3) {
                    break;
                }
            }
        }

        /* Precision 60 FPS pacing */
        clock_gettime(CLOCK_MONOTONIC, &frame_end);
        long elapsed_ns = (frame_end.tv_sec - frame_start.tv_sec) * 1000000000L +
                          (frame_end.tv_nsec - frame_start.tv_nsec);
        long sleep_ns = TARGET_FRAME_NS - elapsed_ns;

        if (sleep_ns > 300000L) { // Sleep if more than 0.3 ms remaining
            struct timespec req = {0, sleep_ns};
            nanosleep(&req, NULL);
        }
    }

    printf("leviathan-art: Exiting graphics scenery (%llu frames rendered at avg %.1f FPS)\n",
           (unsigned long long)frame_count, (double)current_fps);

    /* Free background cache */
    if (s_bg_cache) {
        free(s_bg_cache);
        s_bg_cache = NULL;
    }

    /* Restore terminal settings */
    if (term_set) {
        tcsetattr(STDIN_FILENO, TCSANOW, &orig_term);
    }

    /* Shutdown graphics (restores KD_TEXT so TTY console is normal) */
    graphics_shutdown();
    return 0;
}
