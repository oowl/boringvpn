#include <stdint.h>
#include <linux/if.h>
#include <linux/if_tun.h>
#include <string.h>
#include <sys/ioctl.h>
int32_t setup_dev(int32_t fd,char *ifname,short flags) {
    struct ifreq ifr;
    int err;
    memset(&ifr,0,sizeof(ifr));
    ifr.ifr_flags = flags;
    strncpy(ifr.ifr_name,ifname,IFNAMSIZ);
    if( (err = ioctl(fd, TUNSETIFF, (void *) &ifr)) < 0 ){
        pclose(fd);
        return err;
    }
    strncpy(ifname,ifr.ifr_name,IFNAMSIZ);
    return 0;
}

int32_t setup_tap_device(int32_t fd, char *ifname) {
  return setup_dev(fd, ifname, IFF_TAP | IFF_NO_PI);
}

int32_t setup_tun_device(int32_t fd, char *ifname) {
  return setup_dev(fd, ifname, IFF_TUN | IFF_NO_PI);
}
