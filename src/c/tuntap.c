#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <linux/if.h>
#include <linux/if_tun.h>
#include <string.h>
#include <sys/ioctl.h>
#include <net/route.h>    
#include <netinet/in.h>
#include <arpa/inet.h>   
#include <unistd.h>
int32_t setup_dev(int32_t fd,char *ifname,short flags) {
    struct ifreq ifr;
    int err;
    memset(&ifr,0,sizeof(ifr));
    ifr.ifr_flags = flags;
    strncpy(ifr.ifr_name,ifname,IFNAMSIZ);
    if( (err = ioctl(fd, TUNSETIFF, &ifr)) < 0 ){
        close(fd);
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

int32_t up_device(char *name) {
    struct ifreq ifr;
    int sockfd;
    
    if ((sockfd = socket(AF_INET, SOCK_STREAM, 0)) < 0)
    {
    printf("Create socket fails!\n");
    return -1;	
    }
    
    strncpy(ifr.ifr_name, name,IFNAMSIZ);
    if (ioctl(sockfd, SIOCGIFFLAGS, &ifr) < 0)
    {
    printf("ioctl SIOCGIFFLAGS fails!\n");
    close(sockfd);
    return -1;	
    }
    
    ifr.ifr_flags |= IFF_UP;
    if (ioctl(sockfd, SIOCSIFFLAGS, &ifr) < 0)
    {
    printf("ioctl SIOCSIFFLAGS fails!\n");
    close(sockfd);
    return -1;	
    }
    
    close(sockfd);
    
    return 1;
}

int32_t set_route(char *dev,char *dst_ip, char *mask,char *gateway_addr) {
    struct rtentry route;
    struct sockaddr_in *addr;
    int sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd == -1){
      perror("socket creation failed\n");
      return -1;
    }
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
    route.rt_dev = dev;
    err = ioctl(sockfd, SIOCADDRT, &route);
    if ((err) < 0) {
        return -1;
    }
    return 1;
}


int32_t set_ip(char *name,char *ip_addr,char *netmask) {
    up_device(name);
    int sockfd;
    if ((sockfd = socket(AF_INET, SOCK_STREAM, 0)) < 0)
    {
    printf("Create socket fails!\n");
        return -1;	
    }
    struct ifreq ifr;
    struct sockaddr_in sin;
    sin.sin_family = AF_INET;
    inet_aton(ip_addr,&sin.sin_addr.s_addr);
    memcpy(&ifr.ifr_addr, &sin, sizeof(struct sockaddr)); 
    if (ioctl(sockfd, SIOCSIFADDR, &ifr) < 0) {
        return -1;
    }
    inet_aton(netmask,&sin.sin_addr.s_addr);
    memcpy(&ifr.ifr_netmask, &sin, sizeof(struct sockaddr)); 
    if (ioctl(sockfd, SIOCSIFNETMASK, &ifr) < 0) {
        return -1;
    }
}

// int main() {
//     int fd, err;

//     if( (fd = open("/dev/net/tun", O_RDWR)) < 0 )
//         return -1;
//     char dev[IFNAMSIZ];
//     memset(&dev,0,sizeof(dev));
//     strncpy(dev,"tun1",4);
//     if (setup_tun_device(fd,dev) < 0){
//         printf("setup failed\n");
//         return -1;
//     }
//     if (up_device(dev) < 0){
//         printf("up failed\n");
//         return -1;
//     }
//     if (set_ip(dev,"192.168.1.2","255.255.255.0") < 0){
//         printf("set_ip failed\n");
//         return -1;
//     }
//     if (set_route(dev,"192.168.2.0","255.255.255.0","192.168.1.1") < 0) {
//         printf("set route failed\n");
//         return -1;
//     }
//     sleep(10);
//     close(fd);
//     return 0;
// }