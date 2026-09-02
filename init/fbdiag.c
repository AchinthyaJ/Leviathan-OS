#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <dirent.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <stdint.h>
#include <linux/fb.h>
#include <drm/drm.h>

static void print_banner(void)
{
    printf("====================================================\n");
    printf("        LEVIATHAN GRAPHICS INTERFACE DIAGNOSTIC     \n");
    printf("====================================================\n\n");
}

static void probe_proc_fb(void)
{
    printf("[1] Probing /proc/fb...\n");
    FILE *f = fopen("/proc/fb", "r");
    if (!f) {
        printf("    /proc/fb: not available (%s)\n", strerror(errno));
        return;
    }

    char line[256];
    int count = 0;
    while (fgets(line, sizeof(line), f)) {
        printf("    Registered FB: %s", line);
        count++;
    }
    fclose(f);

    if (count == 0) {
        printf("    /proc/fb is empty (no framebuffer drivers registered).\n");
    }
    printf("\n");
}

static const char *fb_type_str(uint32_t type)
{
    switch (type) {
    case FB_TYPE_PACKED_PIXELS:      return "PACKED_PIXELS";
    case FB_TYPE_PLANES:             return "PLANES";
    case FB_TYPE_INTERLEAVED_PLANES: return "INTERLEAVED_PLANES";
    case FB_TYPE_TEXT:               return "TEXT";
    case FB_TYPE_VGA_PLANES:         return "VGA_PLANES";
    default:                         return "UNKNOWN";
    }
}

static const char *fb_visual_str(uint32_t visual)
{
    switch (visual) {
    case FB_VISUAL_MONO01:           return "MONO01";
    case FB_VISUAL_MONO10:           return "MONO10";
    case FB_VISUAL_TRUECOLOR:        return "TRUECOLOR";
    case FB_VISUAL_PSEUDOCOLOR:      return "PSEUDOCOLOR";
    case FB_VISUAL_DIRECTCOLOR:      return "DIRECTCOLOR";
    case FB_VISUAL_STATIC_PSEUDOCOLOR: return "STATIC_PSEUDOCOLOR";
    default:                         return "UNKNOWN";
    }
}

static void probe_framebuffer(const char *path)
{
    printf("[2] Probing Linux Framebuffer device: %s\n", path);

    if (access(path, F_OK) != 0) {
        printf("    Device node %s does NOT exist: %s\n\n", path, strerror(errno));
        return;
    }
    printf("    Device node %s exists.\n", path);

    int fd = open(path, O_RDWR);
    if (fd < 0) {
        printf("    Failed to open %s in O_RDWR: %s\n", path, strerror(errno));
        fd = open(path, O_RDONLY);
        if (fd < 0) {
            printf("    Failed to open %s in O_RDONLY: %s\n\n", path, strerror(errno));
            return;
        }
        printf("    Opened %s read-only.\n", path);
    } else {
        printf("    Opened %s read-write (fd: %d).\n", path, fd);
    }

    struct fb_fix_screeninfo finfo;
    if (ioctl(fd, FBIOGET_FSCREENINFO, &finfo) < 0) {
        printf("    ioctl FBIOGET_FSCREENINFO failed: %s\n", strerror(errno));
        close(fd);
        return;
    }

    struct fb_var_screeninfo vinfo;
    if (ioctl(fd, FBIOGET_VSCREENINFO, &vinfo) < 0) {
        printf("    ioctl FBIOGET_VSCREENINFO failed: %s\n", strerror(errno));
        close(fd);
        return;
    }

    printf("    --- Fixed Screen Info (fb_fix_screeninfo) ---\n");
    printf("    Device ID:          \"%.*s\"\n", (int)sizeof(finfo.id), finfo.id);
    printf("    SMem Start (Phys):  0x%lx\n", (unsigned long)finfo.smem_start);
    printf("    SMem Length:        %u bytes (%.2f MiB)\n", finfo.smem_len, (double)finfo.smem_len / (1024.0 * 1024.0));
    printf("    Line Length (Pitch):%u bytes\n", finfo.line_length);
    printf("    Type:               %u (%s)\n", finfo.type, fb_type_str(finfo.type));
    printf("    Visual:             %u (%s)\n", finfo.visual, fb_visual_str(finfo.visual));
    printf("    MMIO Start:         0x%lx, Length: %u\n", (unsigned long)finfo.mmio_start, finfo.mmio_len);

    printf("    --- Variable Screen Info (fb_var_screeninfo) ---\n");
    printf("    Visible Resolution: %u x %u\n", vinfo.xres, vinfo.yres);
    printf("    Virtual Resolution: %u x %u\n", vinfo.xres_virtual, vinfo.yres_virtual);
    printf("    Offset:             (%u, %u)\n", vinfo.xoffset, vinfo.yoffset);
    printf("    Bits Per Pixel:     %u bpp (%u bytes per pixel)\n", vinfo.bits_per_pixel, vinfo.bits_per_pixel / 8);
    printf("    Grayscale:          %s\n", vinfo.grayscale ? "yes" : "no");
    printf("    Red   Channel:      offset=%u, length=%u bits, msb_right=%u\n",
           vinfo.red.offset, vinfo.red.length, vinfo.red.msb_right);
    printf("    Green Channel:      offset=%u, length=%u bits, msb_right=%u\n",
           vinfo.green.offset, vinfo.green.length, vinfo.green.msb_right);
    printf("    Blue  Channel:      offset=%u, length=%u bits, msb_right=%u\n",
           vinfo.blue.offset, vinfo.blue.length, vinfo.blue.msb_right);
    printf("    Trans Channel:      offset=%u, length=%u bits, msb_right=%u\n",
           vinfo.transp.offset, vinfo.transp.length, vinfo.transp.msb_right);

    /* Test memory mapping */
    printf("    --- Memory Mapping Test ---\n");
    if (finfo.smem_len == 0) {
        printf("    Warning: smem_len reported 0!\n");
    } else {
        void *mapped = mmap(NULL, finfo.smem_len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        if (mapped == MAP_FAILED) {
            printf("    mmap failed: %s\n", strerror(errno));
        } else {
            printf("    mmap succeeded: mapped %u bytes at address %p\n", finfo.smem_len, mapped);
            munmap(mapped, finfo.smem_len);
        }
    }

    close(fd);
    printf("\n");
}

static void probe_drm(void)
{
    printf("[3] Probing DRM / KMS Subsystem...\n");

    const char *dri_dir = "/dev/dri";
    DIR *d = opendir(dri_dir);
    if (!d) {
        printf("    Directory %s does NOT exist or cannot be opened: %s\n\n", dri_dir, strerror(errno));
        return;
    }

    struct dirent *entry;
    int found_any = 0;
    while ((entry = readdir(d)) != NULL) {
        if (entry->d_name[0] == '.')
            continue;

        char path[512];
        snprintf(path, sizeof(path), "%s/%s", dri_dir, entry->d_name);
        printf("    Found DRM node: %s\n", path);
        found_any++;

        /* If it's a primary node (cardX), probe DRM info */
        if (strncmp(entry->d_name, "card", 4) == 0) {
            int fd = open(path, O_RDWR);
            if (fd < 0) {
                printf("      Could not open %s: %s\n", path, strerror(errno));
                continue;
            }

            char name[128] = {0};
            char date[128] = {0};
            char desc[256] = {0};

            struct drm_version ver;
            memset(&ver, 0, sizeof(ver));
            ver.name = name;
            ver.name_len = sizeof(name) - 1;
            ver.date = date;
            ver.date_len = sizeof(date) - 1;
            ver.desc = desc;
            ver.desc_len = sizeof(desc) - 1;

            if (ioctl(fd, DRM_IOCTL_VERSION, &ver) == 0) {
                printf("      DRM Driver Name:        %s\n", name);
                printf("      DRM Driver Date:        %s\n", date);
                printf("      DRM Driver Description: %s\n", desc);
                printf("      DRM Driver Version:     %d.%d.%d\n", ver.version_major, ver.version_minor, ver.version_patchlevel);
            } else {
                printf("      DRM_IOCTL_VERSION failed: %s\n", strerror(errno));
            }

            struct drm_get_cap cap;
            memset(&cap, 0, sizeof(cap));
            cap.capability = DRM_CAP_DUMB_BUFFER;
            if (ioctl(fd, DRM_IOCTL_GET_CAP, &cap) == 0) {
                printf("      Dumb Buffer Support:    %s (value=%llu)\n", cap.value ? "YES" : "NO", (unsigned long long)cap.value);
            }

            close(fd);
        }
    }
    closedir(d);

    if (!found_any) {
        printf("    No DRM device nodes found in %s.\n", dri_dir);
    }
    printf("\n");
}

static void probe_sysfs(void)
{
    printf("[4] Probing Sysfs Graphics Directories...\n");

    const char *fb_sys = "/sys/class/graphics/fb0";
    if (access(fb_sys, F_OK) == 0) {
        printf("    %s exists.\n", fb_sys);
        char name_path[512];
        snprintf(name_path, sizeof(name_path), "%s/name", fb_sys);
        FILE *f = fopen(name_path, "r");
        if (f) {
            char name[128];
            if (fgets(name, sizeof(name), f)) {
                /* remove newline */
                name[strcspn(name, "\r\n")] = '\0';
                printf("    %s/name: \"%s\"\n", fb_sys, name);
            }
            fclose(f);
        }
    } else {
        printf("    %s does not exist.\n", fb_sys);
    }

    const char *drm_sys = "/sys/class/drm";
    DIR *d = opendir(drm_sys);
    if (d) {
        printf("    %s entries:\n", drm_sys);
        struct dirent *entry;
        while ((entry = readdir(d)) != NULL) {
            if (entry->d_name[0] == '.')
                continue;
            printf("      - %s\n", entry->d_name);
        }
        closedir(d);
    } else {
        printf("    %s does not exist.\n", drm_sys);
    }
    printf("\n");
}

int main(void)
{
    print_banner();
    probe_proc_fb();
    probe_framebuffer("/dev/fb0");
    probe_drm();
    probe_sysfs();
    printf("Diagnostic completed.\n");
    return 0;
}
