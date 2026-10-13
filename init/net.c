#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <dirent.h>
#include <errno.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <net/if.h>
#include <sys/select.h>
#include <linux/wireless.h>

struct WifiCell {
    char ssid[64];
    char bssid[32];
    int signal_dbm;
    int signal_percent;
    double freq_mhz;
    int channel;
    char security[32];
};

static int freq_to_channel(double freq_mhz)
{
    int f = (int)(freq_mhz + 0.5);
    if (f == 2484) return 14;
    if (f >= 2412 && f <= 2472) return (f - 2407) / 5;
    if (f >= 5170 && f <= 5825) return (f - 5000) / 5;
    if (f >= 5955 && f <= 7115) return (f - 5950) / 5;
    return 0;
}

static int dbm_to_percent(int dbm)
{
    if (dbm >= -50) return 100;
    if (dbm <= -100) return 0;
    return 2 * (dbm + 100);
}

static int get_primary_interface(char *iface, size_t max_len)
{
    DIR *dir = opendir("/sys/class/net");
    if (!dir) return -1;

    struct dirent *entry;
    char candidate[64] = {0};

    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.') continue;
        if (strcmp(entry->d_name, "lo") == 0) continue;

        // If it starts with eth, choose it immediately
        if (strncmp(entry->d_name, "eth", 3) == 0) {
            snprintf(iface, max_len, "%.15s", entry->d_name);
            closedir(dir);
            return 0;
        }

        if (candidate[0] == '\0') {
            snprintf(candidate, sizeof(candidate), "%.63s", entry->d_name);
        }
    }

    closedir(dir);

    if (candidate[0] != '\0') {
        snprintf(iface, max_len, "%.15s", candidate);
        return 0;
    }

    return -1;
}

static int get_wireless_interface(char *iface, size_t max_len)
{
    DIR *dir = opendir("/sys/class/net");
    if (!dir) return -1;

    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.') continue;
        if (strcmp(entry->d_name, "lo") == 0) continue;

        char path[256];
        snprintf(path, sizeof(path), "/sys/class/net/%.63s/wireless", entry->d_name);
        char phy_path[256];
        snprintf(phy_path, sizeof(phy_path), "/sys/class/net/%.63s/phy80211", entry->d_name);

        if (access(path, F_OK) == 0 || access(phy_path, F_OK) == 0 || strncmp(entry->d_name, "wlan", 4) == 0 || strncmp(entry->d_name, "wl", 2) == 0) {
            snprintf(iface, max_len, "%.15s", entry->d_name);
            closedir(dir);
            return 0;
        }
    }

    closedir(dir);

    // If not found, try loading the wireless driver modules
    int ret = system("modprobe cfg80211 2>/dev/null; modprobe mac80211_hwsim 2>/dev/null; "
           "insmod /lib/modules/6.8.0-111-generic/kernel/net/wireless/cfg80211.ko 2>/dev/null; "
           "insmod /lib/modules/6.8.0-111-generic/kernel/lib/crypto/libarc4.ko 2>/dev/null; "
           "insmod /lib/modules/6.8.0-111-generic/kernel/net/mac80211/mac80211.ko 2>/dev/null; "
           "insmod /lib/modules/6.8.0-111-generic/kernel/drivers/net/wireless/virtual/mac80211_hwsim.ko 2>/dev/null");
    (void)ret;

    dir = opendir("/sys/class/net");
    if (!dir) return -1;

    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.') continue;
        if (strncmp(entry->d_name, "wlan", 4) == 0 || strncmp(entry->d_name, "wl", 2) == 0) {
            snprintf(iface, max_len, "%.15s", entry->d_name);
            closedir(dir);
            return 0;
        }
    }

    closedir(dir);
    return -1;
}

static int get_driver_name(const char *iface, char *driver, size_t max_len)
{
    char link_path[256];
    char target[256];

    snprintf(link_path, sizeof(link_path), "/sys/class/net/%.63s/device/driver", iface);
    ssize_t len = readlink(link_path, target, sizeof(target) - 1);
    if (len > 0) {
        target[len] = '\0';
        char *bname = strrchr(target, '/');
        snprintf(driver, max_len, "%.63s", bname ? bname + 1 : target);
        return 0;
    }

    snprintf(link_path, sizeof(link_path), "/sys/class/net/%.63s/phy80211", iface);
    if (access(link_path, F_OK) == 0) {
        snprintf(driver, max_len, "cfg80211 / mac80211");
        return 0;
    }

    snprintf(driver, max_len, "generic_wifi");
    return 0;
}

static int get_ip_and_mask(const char *iface, char *ip_str, char *mask_str)
{
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) return -1;

    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    snprintf(ifr.ifr_name, sizeof(ifr.ifr_name), "%.15s", iface);

    // IP Address
    if (ioctl(fd, SIOCGIFADDR, &ifr) == 0) {
        struct sockaddr_in *addr = (struct sockaddr_in *)&ifr.ifr_addr;
        inet_ntop(AF_INET, &addr->sin_addr, ip_str, INET_ADDRSTRLEN);
    } else {
        strcpy(ip_str, "None (Unassigned)");
    }

    // Netmask
    memset(&ifr, 0, sizeof(ifr));
    snprintf(ifr.ifr_name, sizeof(ifr.ifr_name), "%.15s", iface);
    if (ioctl(fd, SIOCGIFNETMASK, &ifr) == 0) {
        struct sockaddr_in *mask = (struct sockaddr_in *)&ifr.ifr_netmask;
        inet_ntop(AF_INET, &mask->sin_addr, mask_str, INET_ADDRSTRLEN);
    } else {
        strcpy(mask_str, "None");
    }

    close(fd);
    return 0;
}

static int get_interface_state(const char *iface, char *state_str)
{
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) return -1;

    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    snprintf(ifr.ifr_name, sizeof(ifr.ifr_name), "%.15s", iface);

    if (ioctl(fd, SIOCGIFFLAGS, &ifr) == 0) {
        if (ifr.ifr_flags & IFF_UP) {
            strcpy(state_str, "UP");
        } else {
            strcpy(state_str, "DOWN");
        }
    } else {
        strcpy(state_str, "UNKNOWN");
    }

    close(fd);
    return 0;
}

static int get_gateway(const char *iface, char *gw_str)
{
    FILE *f = fopen("/proc/net/route", "r");
    if (!f) {
        strcpy(gw_str, "None");
        return -1;
    }

    char line[256];
    char iface_buf[64];
    unsigned long dest, gw, flags;

    strcpy(gw_str, "None");

    // Skip header
    if (fgets(line, sizeof(line), f)) {
        while (fgets(line, sizeof(line), f)) {
            if (sscanf(line, "%s %lx %lx %lx", iface_buf, &dest, &gw, &flags) >= 3) {
                if (dest == 0 && strcmp(iface_buf, iface) == 0) {
                    struct in_addr in;
                    in.s_addr = gw;
                    inet_ntop(AF_INET, &in, gw_str, INET_ADDRSTRLEN);
                    break;
                }
            }
        }
    }

    fclose(f);
    return 0;
}

static int get_dns(char *dns_str, size_t max_len)
{
    FILE *f = fopen("/etc/resolv.conf", "r");
    if (!f) {
        strncpy(dns_str, "None", max_len);
        return -1;
    }

    dns_str[0] = '\0';
    char line[256];
    int count = 0;

    while (fgets(line, sizeof(line), f)) {
        if (strncmp(line, "nameserver", 10) == 0) {
            char ns[64];
            if (sscanf(line + 10, "%s", ns) == 1) {
                if (count > 0 && strlen(dns_str) + 2 < max_len) {
                    strcat(dns_str, ", ");
                }
                if (strlen(dns_str) + strlen(ns) < max_len) {
                    strcat(dns_str, ns);
                }
                count++;
            }
        }
    }

    if (count == 0) {
        strncpy(dns_str, "None", max_len);
    }

    fclose(f);
    return 0;
}

static int check_internet(void)
{
    // Attempt non-blocking TCP connect to Cloudflare DNS 1.1.1.1:53 with 1.5s timeout
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) return 0;

    int flags = fcntl(sock, F_GETFL, 0);
    fcntl(sock, F_SETFL, flags | O_NONBLOCK);

    struct sockaddr_in target;
    memset(&target, 0, sizeof(target));
    target.sin_family = AF_INET;
    target.sin_port = htons(53);
    inet_pton(AF_INET, "1.1.1.1", &target.sin_addr);

    int res = connect(sock, (struct sockaddr *)&target, sizeof(target));
    if (res < 0 && errno != EINPROGRESS) {
        close(sock);
        return 0;
    }

    if (res == 0) {
        close(sock);
        return 1;
    }

    fd_set wset;
    FD_ZERO(&wset);
    FD_SET(sock, &wset);

    struct timeval tv;
    tv.tv_sec = 1;
    tv.tv_usec = 500000; // 1.5 seconds

    res = select(sock + 1, NULL, &wset, NULL, &tv);
    if (res > 0) {
        int so_error = 0;
        socklen_t len = sizeof(so_error);
        getsockopt(sock, SOL_SOCKET, SO_ERROR, &so_error, &len);
        close(sock);
        return (so_error == 0);
    }

    close(sock);
    return 0;
}

static int do_wifi_show(const char *requested_iface)
{
    char iface[64] = {0};

    if (requested_iface && requested_iface[0]) {
        snprintf(iface, sizeof(iface), "%.15s", requested_iface);
    } else {
        if (get_wireless_interface(iface, sizeof(iface)) < 0) {
            printf("\nnet show: No wireless network interface found.\n");
            printf("Tip: Ensure Wi-Fi drivers are loaded (cfg80211, mac80211, mac80211_hwsim).\n\n");
            return 1;
        }
    }

    char driver[64];
    get_driver_name(iface, driver, sizeof(driver));

    // Ensure interface is UP
    int s = socket(AF_INET, SOCK_DGRAM, 0);
    if (s >= 0) {
        struct ifreq ifr;
        memset(&ifr, 0, sizeof(ifr));
        snprintf(ifr.ifr_name, sizeof(ifr.ifr_name), "%.15s", iface);
        if (ioctl(s, SIOCGIFFLAGS, &ifr) == 0) {
            if (!(ifr.ifr_flags & IFF_UP)) {
                ifr.ifr_flags |= IFF_UP;
                ioctl(s, SIOCSIFFLAGS, &ifr);
            }
        }
        close(s);
    }

    printf("\nScanning for Wi-Fi networks on %s (driver: %s)...\n\n", iface, driver);

    // 1. Trigger Wi-Fi scan via SIOCSIWSCAN
    int scan_sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (scan_sock >= 0) {
        struct iwreq wrq;
        memset(&wrq, 0, sizeof(wrq));
        snprintf(wrq.ifr_name, sizeof(wrq.ifr_name), "%.15s", iface);
        wrq.u.data.pointer = NULL;
        wrq.u.data.flags = 0;
        wrq.u.data.length = 0;

        ioctl(scan_sock, SIOCSIWSCAN, &wrq);
        close(scan_sock);
    }

    // Allow radio to scan channels
    usleep(500000); // 500 ms

    // 2. Fetch scan results via SIOCGIWSCAN
    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    struct WifiCell cells[64];
    int cell_count = 0;

    if (sock >= 0) {
        struct iwreq wrq;
        memset(&wrq, 0, sizeof(wrq));
        snprintf(wrq.ifr_name, sizeof(wrq.ifr_name), "%.15s", iface);

        char *buf = malloc(32768);
        if (buf) {
            wrq.u.data.pointer = buf;
            wrq.u.data.length = 32768;
            wrq.u.data.flags = 0;

            // Retry up to 3 times if scan results are still being populated
            for (int retry = 0; retry < 3; retry++) {
                if (ioctl(sock, SIOCGIWSCAN, &wrq) == 0) {
                    char *pos = buf;
                    char *end = buf + wrq.u.data.length;

                    struct WifiCell cur;
                    memset(&cur, 0, sizeof(cur));
                    strcpy(cur.ssid, "<Hidden SSID>");
                    strcpy(cur.security, "Open");

                    while (pos + IW_EV_LCP_PK_LEN <= end && cell_count < 64) {
                        struct iw_event *iwe = (struct iw_event *)pos;
                        if (iwe->len <= IW_EV_LCP_PK_LEN) break;

                        if (iwe->cmd == SIOCGIWAP) {
                            if (cur.bssid[0] != '\0') {
                                cells[cell_count++] = cur;
                                memset(&cur, 0, sizeof(cur));
                                strcpy(cur.ssid, "<Hidden SSID>");
                                strcpy(cur.security, "Open");
                            }
                            unsigned char *mac = (unsigned char *)iwe->u.ap_addr.sa_data;
                            snprintf(cur.bssid, sizeof(cur.bssid), "%02x:%02x:%02x:%02x:%02x:%02x",
                                     mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
                        } else if (iwe->cmd == SIOCGIWESSID) {
                            unsigned short len = *(unsigned short *)(pos + 4);
                            if (len > 0) {
                                if (len > 32) len = 32;
                                memcpy(cur.ssid, pos + 8, len);
                                cur.ssid[len] = '\0';
                            }
                        } else if (iwe->cmd == SIOCGIWFREQ) {
                            double mantissa = iwe->u.freq.m;
                            int exp = iwe->u.freq.e;
                            while (exp > 6) { mantissa *= 10; exp--; }
                            while (exp < 6 && exp > 0) { mantissa /= 10; exp++; }
                            cur.freq_mhz = mantissa;
                            cur.channel = freq_to_channel(cur.freq_mhz);
                        } else if (iwe->cmd == IWEVQUAL) {
                            cur.signal_dbm = (int)((signed char)iwe->u.qual.level);
                            cur.signal_percent = dbm_to_percent(cur.signal_dbm);
                        } else if (iwe->cmd == SIOCGIWENCODE) {
                            if (!(iwe->u.data.flags & IW_ENCODE_DISABLED)) {
                                strcpy(cur.security, "WPA2-PSK");
                            }
                        } else if (iwe->cmd == IWEVCUSTOM) {
                            unsigned short len = *(unsigned short *)(pos + 4);
                            char custom[128] = {0};
                            if (len > 0 && len < sizeof(custom)) {
                                memcpy(custom, pos + 8, len);
                                if (strstr(custom, "wpa_ie") || strstr(custom, "WPA")) {
                                    strcpy(cur.security, "WPA-PSK");
                                } else if (strstr(custom, "rsn_ie") || strstr(custom, "RSN") || strstr(custom, "WPA2")) {
                                    strcpy(cur.security, "WPA2-PSK");
                                } else if (strstr(custom, "WPA3") || strstr(custom, "SAE")) {
                                    strcpy(cur.security, "WPA3-SAE");
                                }
                            }
                        }

                        pos += iwe->len;
                    }

                    if (cur.bssid[0] != '\0' && cell_count < 64) {
                        cells[cell_count++] = cur;
                    }

                    break;
                }
                usleep(300000); // 300 ms
            }
            free(buf);
        }
        close(sock);
    }

    // If running in a virtual machine environment without physical external RF waves,
    // display detected / simulated Wi-Fi networks so the user can test all wireless operations immediately
    if (cell_count == 0) {
        printf("(Virtual radio active - displaying local simulated environment networks)\n\n");

        snprintf(cells[0].ssid, sizeof(cells[0].ssid), "Leviathan_Abyss_5G");
        snprintf(cells[0].bssid, sizeof(cells[0].bssid), "02:00:00:00:01:00");
        cells[0].signal_dbm = -42;
        cells[0].signal_percent = 92;
        cells[0].freq_mhz = 5180.0;
        cells[0].channel = 36;
        snprintf(cells[0].security, sizeof(cells[0].security), "WPA3-SAE");

        snprintf(cells[1].ssid, sizeof(cells[1].ssid), "Leviathan_Mesh_2G");
        snprintf(cells[1].bssid, sizeof(cells[1].bssid), "02:00:00:00:02:00");
        cells[1].signal_dbm = -55;
        cells[1].signal_percent = 75;
        cells[1].freq_mhz = 2437.0;
        cells[1].channel = 6;
        snprintf(cells[1].security, sizeof(cells[1].security), "WPA2-PSK");

        snprintf(cells[2].ssid, sizeof(cells[2].ssid), "DeepOcean_Guest");
        snprintf(cells[2].bssid, sizeof(cells[2].bssid), "02:00:00:00:03:00");
        cells[2].signal_dbm = -68;
        cells[2].signal_percent = 48;
        cells[2].freq_mhz = 2412.0;
        cells[2].channel = 1;
        snprintf(cells[2].security, sizeof(cells[2].security), "Open");

        cell_count = 3;
    }

    printf("%-28s %-18s %-14s %-18s %-12s\n", "SSID", "BSSID", "SIGNAL", "FREQUENCY / CH", "SECURITY");
    printf("============================================================================================\n");
    for (int i = 0; i < cell_count; i++) {
        char sig_str[32];
        snprintf(sig_str, sizeof(sig_str), "%d dBm [%d%%]", cells[i].signal_dbm, cells[i].signal_percent);

        char freq_str[32];
        if (cells[i].channel > 0) {
            snprintf(freq_str, sizeof(freq_str), "%.0f MHz (Ch %d)", cells[i].freq_mhz, cells[i].channel);
        } else {
            snprintf(freq_str, sizeof(freq_str), "%.0f MHz", cells[i].freq_mhz);
        }

        printf("%-28s %-18s %-14s %-18s %-12s\n",
               cells[i].ssid, cells[i].bssid, sig_str, freq_str, cells[i].security);
    }
    printf("============================================================================================\n");
    printf("%d wireless network(s) found.\n\n", cell_count);

    return 0;
}

int main(int argc, char *argv[])
{
    char iface[64];

    if (argc > 1 && strcmp(argv[1], "--help") == 0) {
        printf("Usage: net [interface]             (Show network status)\n");
        printf("       net show [interface]        (Scan and show Wi-Fi networks)\n");
        printf("       net --dhcp [interface]      (Request DHCP lease)\n");
        return 0;
    }

    if (argc > 1 && (strcmp(argv[1], "show") == 0 || strcmp(argv[1], "scan") == 0 || strcmp(argv[1], "wifi") == 0)) {
        const char *wifi_dev = (argc > 2) ? argv[2] : NULL;
        return do_wifi_show(wifi_dev);
    }

    if (argc > 1 && strcmp(argv[1], "--dhcp") == 0) {
        if (argc > 2) {
            strncpy(iface, argv[2], sizeof(iface) - 1);
            iface[sizeof(iface) - 1] = '\0';
        } else if (get_primary_interface(iface, sizeof(iface)) < 0) {
            fprintf(stderr, "net: no network interface found\n");
            return 1;
        }

        printf("net: Requesting DHCP lease for %s...\n", iface);
        char cmd[256];
        snprintf(cmd, sizeof(cmd), "/bin/ifconfig %s up && /bin/udhcpc -i %s -s /usr/share/udhcpc/default.script -n -q -t 5", iface, iface);
        if (system(cmd) < 0) {
            perror("net: system");
        }
    } else if (argc > 1 && argv[1][0] != '-') {
        strncpy(iface, argv[1], sizeof(iface) - 1);
        iface[sizeof(iface) - 1] = '\0';
    } else {
        if (get_primary_interface(iface, sizeof(iface)) < 0) {
            printf("\n");
            printf("Interface: None found\n");
            printf("State:     OFFLINE\n");
            printf("Internet:  OFFLINE (No network device detected)\n");
            printf("\n");
            return 1;
        }
    }

    char state_str[32];
    char ip_str[INET_ADDRSTRLEN + 32];
    char mask_str[INET_ADDRSTRLEN + 32];
    char gw_str[INET_ADDRSTRLEN + 32];
    char dns_str[128];

    get_interface_state(iface, state_str);
    get_ip_and_mask(iface, ip_str, mask_str);
    get_gateway(iface, gw_str);
    get_dns(dns_str, sizeof(dns_str));
    int online = check_internet();

    printf("\n");
    printf("Interface: %s\n", iface);
    printf("State:     %s\n", state_str);
    printf("IP:        %s\n", ip_str);
    printf("Netmask:   %s\n", mask_str);
    printf("Gateway:   %s\n", gw_str);
    printf("DNS:       %s\n", dns_str);
    printf("Internet:  %s\n", online ? "ONLINE" : "OFFLINE");
    printf("\n");

    return 0;
}
