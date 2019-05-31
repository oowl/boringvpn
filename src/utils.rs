use std::process;
use libc;
use log::info;


pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}
pub fn enable_ipv4_forwarding() -> Result<(),String> {
    let sysctl_arg = "net.ipv4.ip_forward=1";
    info!("Enable IPv4 Forwarding");
    let status = process::Command::new("sysctl")
        .arg("-w")
        .arg(sysctl_arg)
        .status()
        .unwrap();
    if status.success() {
        Ok(())
    } else {
        Err(format!("sysctl: {}",status))
    }
}

pub fn get_default_gateway() -> Result<String,String> {
    let cmd = "ip -4 route list 0/0 | awk '{print $3}'";
    let output = process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .unwrap();
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap().trim_right().to_string())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }
}

pub fn add_route(route_type: RouteType,route: &str,gateway: &str) -> Result<(),String> {
    let mode = match route_type {
        RouteType::Net => "-net",
        RouteType::Host => "-host"
    };
    info!("Adding route: {} {} gateway {}",mode,route,gateway);
    let status = process::Command::new("route")
        .arg("-n")
        .arg("add")
        .arg(mode)
        .arg(route)
        .arg("gw")
        .arg(gateway)
        .status()
        .unwrap();
    if status.success() {
        Ok(())
    } else {
        Err(format!("route: {}",status))
    }
}

pub fn delete_route(route_type: RouteType,route: &str) -> Result<(),String> {
    let mode = match route_type {
        RouteType::Net => "-net",
        RouteType::Host => "-host"
    };
    info!("Deleting route: {} {}",mode,route);
    let status = process::Command::new("route")
        .arg("-n")
        .arg("del")
        .arg(mode)
        .arg(route)
        .status()
        .unwrap();
    if status.success() {
        Ok(())
    } else {
        Err(format!("route: {}",status))
    }
}

pub fn set_default_gateway(gateway: &str) -> Result<(),String> {
    add_route(RouteType::Net, "default", gateway)
}

pub fn delete_default_gateway() -> Result<(),String> {
    delete_route(RouteType::Net, "default")
}

pub enum RouteType {
    Net,
    Host,
}

pub struct DefaultGateWay {
    origin: String,
    remote: String,
}

impl DefaultGateWay {
    pub fn create(gateway: &str,remote: &str) -> DefaultGateWay {
        let origin = get_default_gateway().unwrap();
        info!("Original default gateway: {}",origin);
        add_route(RouteType::Host, remote, &origin).unwrap();
        delete_default_gateway().unwrap();
        set_default_gateway(gateway).unwrap();
        DefaultGateWay {
            origin: origin,
            remote: String::from(remote)
        }
    }
}

impl Drop for DefaultGateWay {
    fn drop(&mut self) {
        delete_default_gateway().unwrap();
        set_default_gateway(&self.origin).unwrap();
        delete_route(RouteType::Host, &self.remote).unwrap();
    }
}

fn get_route_gateway(route: &str) -> Result<String,String> {
    let cmd = format!("ip -4 route list {}",route);
    let output = process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .unwrap();
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap().trim_right().to_string())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }

}

pub fn get_public_ip() -> Result<String, String> {
    let output = process::Command::new("curl")
        .arg("ipecho.net/plain")
        .output()
        .unwrap();
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }
}

pub fn set_dns(dns: &str) -> Result<String,String> {
    let cmd = format!("echo nameserver {} > /etc/resolv.conf",dns);
    let output = process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .unwrap();
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::*;
    
    #[test]
    fn get_default_gateway_test() {
        let a = get_default_gateway().unwrap();
        assert!(get_route_gateway("0/0").unwrap().contains(&*a))
    }

    #[test]
    fn route_test() {
        assert!(is_root());
        let gw = get_default_gateway().unwrap();
        add_route(RouteType::Host, "1.1.1.1", &gw).unwrap();
        assert!(get_route_gateway("1.1.1.1").unwrap().contains(&*gw));
        delete_route(RouteType::Host, "1.1.1.1").unwrap();
        assert!(!get_route_gateway("1.1.1.1").unwrap().contains(&*gw));
    }
}