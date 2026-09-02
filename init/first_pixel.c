#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <linux/fb.h>

int main(void)
{
    printf("========================================\n");
    printf("     LEVIATHAN FIRST PIXEL PROGRAM      \n");
    printf("========================================\n");

    /* 1. Open the Linux graphics device */
    const char *fb_path = "/dev/fb0";
    int fb_fd = open(fb_path, O_RDWR);
    if (fb_fd < 0) {
        perror("first_pixel: failed to open /dev/fb0");
        return 1;
    }
    printf("[1] Opened %s (fd: %d)\n", fb_path, fb_fd);

    /* 2. Obtain screen dimensions & buffer configuration */
    struct fb_fix_screeninfo finfo;
    if (ioctl(fb_fd, FBIOGET_FSCREENINFO, &finfo) < 0) {
        perror("first_pixel: FBIOGET_FSCREENINFO failed");
        close(fb_fd);
        return 1;
    }

    struct fb_var_screeninfo vinfo;
    if (ioctl(fb_fd, FBIOGET_VSCREENINFO, &vinfo) < 0) {
        perror("first_pixel: FBIOGET_VSCREENINFO failed");
        close(fb_fd);
        return 1;
    }

    printf("[2] Screen dimensions: %u x %u (pitch: %u bytes, buffer size: %u bytes)\n",
           vinfo.xres, vinfo.yres, finfo.line_length, finfo.smem_len);

    /* 3. Obtain pixel format and bit layout */
    printf("[3] Pixel layout: %u bpp\n", vinfo.bits_per_pixel);
    printf("    Red:   offset %u, length %u\n", vinfo.red.offset, vinfo.red.length);
    printf("    Green: offset %u, length %u\n", vinfo.green.offset, vinfo.green.length);
    printf("    Blue:  offset %u, length %u\n", vinfo.blue.offset, vinfo.blue.length);

    if (vinfo.bits_per_pixel != 32 && vinfo.bits_per_pixel != 24 && vinfo.bits_per_pixel != 16) {
        fprintf(stderr, "first_pixel: unsupported bpp (%u)\n", vinfo.bits_per_pixel);
        close(fb_fd);
        return 1;
    }

    /* 4. Memory-map the framebuffer */
    uint8_t *fb_mem = (uint8_t *)mmap(NULL, finfo.smem_len, PROT_READ | PROT_WRITE, MAP_SHARED, fb_fd, 0);
    if (fb_mem == MAP_FAILED) {
        perror("first_pixel: mmap failed");
        close(fb_fd);
        return 1;
    }
    printf("[4] Framebuffer memory mapped successfully at %p\n", (void *)fb_mem);

    /* 5. Draw ONE visible pixel at center of the screen */
    uint32_t target_x = vinfo.xres / 2;
    uint32_t target_y = vinfo.yres / 2;

    /* Bounds check */
    if (target_x >= vinfo.xres || target_y >= vinfo.yres) {
        fprintf(stderr, "first_pixel: coordinate (%u, %u) out of bounds (%u, %u)\n",
                target_x, target_y, vinfo.xres, vinfo.yres);
        munmap(fb_mem, finfo.smem_len);
        close(fb_fd);
        return 1;
    }

    /* Format pixel color: Bright Pure Red (R=255, G=0, B=0) */
    uint8_t r = 255, g = 0, b = 0;
    uint32_t pixel_value = ((r >> (8 - vinfo.red.length)) << vinfo.red.offset) |
                           ((g >> (8 - vinfo.green.length)) << vinfo.green.offset) |
                           ((b >> (8 - vinfo.blue.length)) << vinfo.blue.offset);

    /* Calculate byte offset using pitch (finfo.line_length) and bytes per pixel */
    uint32_t bytes_per_pixel = vinfo.bits_per_pixel / 8;
    size_t byte_offset = (size_t)target_y * finfo.line_length + (size_t)target_x * bytes_per_pixel;

    if (byte_offset + bytes_per_pixel > finfo.smem_len) {
        fprintf(stderr, "first_pixel: byte offset %zu exceeds smem_len %u\n",
                byte_offset, finfo.smem_len);
        munmap(fb_mem, finfo.smem_len);
        close(fb_fd);
        return 1;
    }

    /* Write pixel */
    if (bytes_per_pixel == 4) {
        *(volatile uint32_t *)(fb_mem + byte_offset) = pixel_value;
    } else if (bytes_per_pixel == 2) {
        *(volatile uint16_t *)(fb_mem + byte_offset) = (uint16_t)pixel_value;
    } else if (bytes_per_pixel == 3) {
        fb_mem[byte_offset + 0] = b;
        fb_mem[byte_offset + 1] = g;
        fb_mem[byte_offset + 2] = r;
    }

    /* Read-back verification */
    uint32_t read_back = 0;
    if (bytes_per_pixel == 4) {
        read_back = *(volatile uint32_t *)(fb_mem + byte_offset);
    }

    printf("[5] Pixel written to (%u, %u):\n", target_x, target_y);
    printf("    Color:       R=%u, G=%u, B=%u -> Encoded: 0x%08X\n", r, g, b, pixel_value);
    printf("    Byte Offset: %zu (0x%zX)\n", byte_offset, byte_offset);
    printf("    Read Back:   0x%08X (matches: %s)\n", read_back,
           (read_back == pixel_value) ? "YES" : "NO");

    /* 6. Keep display alive long enough to verify visually */
    printf("[6] Keeping display alive for verification (1 second)...\n");
    sleep(1);

    /* Unmap and close */
    munmap(fb_mem, finfo.smem_len);
    close(fb_fd);
    printf("Done.\n");

    return 0;
}
