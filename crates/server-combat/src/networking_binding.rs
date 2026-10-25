use std::net::{IpAddr, Ipv4Addr, SocketAddr};

const DEFAULT_PORT: u16 = 5000;

pub(crate) fn parse_server_address(
    bind_ip: Option<&str>,
    port: Option<&str>,
) -> Result<SocketAddr, String> {
    let ip = match bind_ip {
        Some(value) => value
            .parse::<IpAddr>()
            .map_err(|error| format!("invalid GAME_SERVER_BIND_IP={value:?}: {error}"))?,
        None => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    };
    let port = match port {
        Some(value) => value
            .parse::<u16>()
            .map_err(|error| format!("invalid GAME_SERVER_PORT={value:?}: {error}"))?,
        None => DEFAULT_PORT,
    };
    if port == 0 {
        return Err("GAME_SERVER_PORT must be greater than zero".into());
    }
    Ok(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_server_binds_only_explicit_loopback() {
        assert_eq!(
            parse_server_address(Some("127.0.0.1"), Some("5098")).unwrap(),
            "127.0.0.1:5098".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(
            parse_server_address(Some("::1"), Some("5098")).unwrap(),
            "[::1]:5098".parse::<SocketAddr>().unwrap()
        );
    }

    #[test]
    fn invalid_binding_configuration_fails_instead_of_using_defaults() {
        for (ip, port) in [
            (Some("not-an-ip"), Some("5098")),
            (Some("127.0.0.1"), Some("0")),
            (None, Some("invalid")),
            (None, Some("65536")),
        ] {
            assert!(
                parse_server_address(ip, port).is_err(),
                "invalid {ip:?}/{port:?} accepted"
            );
        }
    }

    #[test]
    fn unset_binding_configuration_keeps_standard_server_address() {
        assert_eq!(
            parse_server_address(None, None).unwrap(),
            "0.0.0.0:5000".parse::<SocketAddr>().unwrap()
        );
    }
}
