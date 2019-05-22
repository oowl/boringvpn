#include <stdint.h>
#include <linux/if.h>
#include <linux/if_tun.h>
#include <string.h>
#include <sys/ioctl.h>
#include <net/route.h>    
#include <netinet/in.h>   
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

int32_t set_route(int sockfd, char *gateway_addr, char *dst_ip, char *mask) {
    struct rtentry route;
    struct sockaddr_in *addr;
    int err = 0;
    memset(&route, 0, sizeof(route));
    addr = (struct sockaddr_in*) &route.rt_gateway;
    addr->sin_family = AF_INET;
    addr->sin_addr.s_addr = inet_addr(gateway_addr);
    addr = (struct sockaddr_in*) &route.rt_dst;
    addr->sin_family = AF_INET;
    addr->sin_addr.s_addr = inet_addr(dst_ip);
    addr = (struct sockaddr_in*) &route.rt_genmask;
    addr->sin_family = AF_INET;
    addr->sin_addr.s_addr = inet_addr(mask);
    route.rt_flags = RTF_UP | RTF_GATEWAY;
    route.rt_metric = 100;
    err = ioctl(sockfd, SIOCADDRT, &route);
    if ((err) < 0) {
    return -1;
    }
    return 1;
}

int set_ip(char *iface_name, char *ip_addr, char *gateway_addr)
{
    if(!iface_name)
    return -1;
    struct ifreq ifr;
    struct sockaddr_in sin;
    int sockfd = create_socket();

    sin.sin_family = AF_INET;

    // Convert IP from numbers and dots to binary notation
    inet_aton(ip_addr,&sin.sin_addr.s_addr);

    /* get interface name */
    strncpy(ifr.ifr_name, iface_name, IFNAMSIZ);

    /* Read interface flags */
    ioctl(sockfd,SIOCGIFFLAGS,&ifr);
    /*
    * Expected in <net/if.h> according to
    * "UNIX Network Programming".
    */
    #ifdef ifr_flags
    # define IRFFLAGS       ifr_flags
    #else   /* Present on kFreeBSD */
    # define IRFFLAGS       ifr_flagshigh
    #endif
    // If interface is down, bring it up
    if (ifr.IRFFLAGS | ~(IFF_UP)) {
        ifr.IRFFLAGS |= IFF_UP;
        ioctl(sockfd,SIOCSIFFLAGS,&ifr);
    }
    // Set route
    // set_route(sockfd, gateway_addr);
    memcpy(&ifr.ifr_addr, &sin, sizeof(struct sockaddr)); 
    // Set interface address
    if (ioctl(sockfd, SIOCSIFADDR, &ifr) < 0) {
        return -1;
    }             
    #undef IRFFLAGS 

    return 0;
}