use std::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }
}

/// An absolute `http://` or `https://` URL: `scheme://host[:port][/path][?query]`.
/// ASCII only, so internationalized domains must be written in their `xn--`
/// form. No user info and no `#fragment`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Url {
    pub scheme: Scheme,
    /// IPv6 addresses without the brackets.
    pub host: String,
    pub port: u16,
    /// Path and query, always starting with `/`.
    pub target: String,
}

impl Url {
    /// The `Host` header value: the port is left out when it's the default.
    pub fn authority(&self) -> String {
        authority(&self.host, self.port, self.scheme.default_port())
    }
}

fn authority(host: &str, port: u16, default_port: u16) -> String {
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    if port == default_port {
        host
    } else {
        format!("{host}:{port}")
    }
}

/// A TLS endpoint written as `host` (port 443) or `host:port`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct HostPort {
    pub host: String,
    pub port: u16,
}

impl TryFrom<String> for HostPort {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let error = |reason| format!("\"{s}\" is not a valid host: {reason}");
        if s.contains(['/', '?']) {
            return Err(error(
                "expected host or host:port, without a scheme or path",
            ));
        }
        let url = parse(&format!("https://{s}")).map_err(error)?;
        Ok(Self {
            host: url.host,
            port: url.port,
        })
    }
}

impl fmt::Display for HostPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let default_port = Scheme::Https.default_port();
        write!(f, "{}", authority(&self.host, self.port, default_port))
    }
}

impl TryFrom<String> for Url {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        parse(&s).map_err(|reason| format!("\"{s}\" is not a valid URL: {reason}"))
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scheme = match self.scheme {
            Scheme::Http => "http",
            Scheme::Https => "https",
        };
        write!(f, "{scheme}://{}{}", self.authority(), self.target)
    }
}

fn parse(s: &str) -> Result<Url, &'static str> {
    if !s.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("only printable ASCII without spaces is allowed");
    }
    if s.contains('#') {
        return Err("fragments (#...) are not sent to servers");
    }
    let (scheme, rest) = s.split_once("://").ok_or("expected http:// or https://")?;
    let scheme = match scheme.to_ascii_lowercase().as_str() {
        "http" => Scheme::Http,
        "https" => Scheme::Https,
        _ => return Err("expected http:// or https://"),
    };
    let target_start = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, target) = rest.split_at(target_start);
    if authority.contains('@') {
        return Err("user info (user@host) is not supported");
    }
    let (host, port) = split_port(authority)?;
    let port = match port {
        Some(port) => port
            .parse::<u16>()
            .ok()
            .filter(|&p| p != 0)
            .ok_or("bad port")?,
        None => scheme.default_port(),
    };
    let target = match target.strip_prefix('?') {
        Some(_) => format!("/{target}"),
        None if target.is_empty() => "/".to_owned(),
        None => target.to_owned(),
    };
    Ok(Url {
        scheme,
        host: host.to_ascii_lowercase(),
        port,
        target,
    })
}

/// `host`, `host:port`, `[v6]` or `[v6]:port`.
fn split_port(authority: &str) -> Result<(&str, Option<&str>), &'static str> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest.split_once(']').ok_or("unclosed [ in host")?;
        if host.is_empty() || !host.bytes().all(|b| b.is_ascii_hexdigit() || b == b':') {
            return Err("bad IPv6 address");
        }
        return match after {
            "" => Ok((host, None)),
            _ => Ok((host, Some(after.strip_prefix(':').ok_or("bad port")?))),
        };
    }
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    let valid = !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
    if !valid {
        return Err("bad host");
    }
    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::try_from(s.to_owned()).unwrap()
    }

    #[test]
    fn parses_parts() {
        let u = url("https://Example.com/api/health?x=1");
        assert_eq!(u.scheme, Scheme::Https);
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 443);
        assert_eq!(u.target, "/api/health?x=1");

        let u = url("http://127.0.0.1:5757");
        assert_eq!(
            (u.host.as_str(), u.port, u.target.as_str()),
            ("127.0.0.1", 5757, "/")
        );

        let u = url("http://[::1]:8080?q");
        assert_eq!(
            (u.host.as_str(), u.port, u.target.as_str()),
            ("::1", 8080, "/?q")
        );
    }

    #[test]
    fn formats_authority_and_url() {
        assert_eq!(url("https://example.com:443/").authority(), "example.com");
        assert_eq!(
            url("http://example.com:8080/a").authority(),
            "example.com:8080"
        );
        assert_eq!(url("http://[::1]:8080/").authority(), "[::1]:8080");
        assert_eq!(
            url("https://example.com").to_string(),
            "https://example.com/"
        );
    }

    #[test]
    fn parses_host_ports() {
        let parse = |s: &str| HostPort::try_from(s.to_owned());
        let hp = parse("Example.com").unwrap();
        assert_eq!((hp.host.as_str(), hp.port), ("example.com", 443));
        assert_eq!(hp.to_string(), "example.com");
        let hp = parse("example.com:8443").unwrap();
        assert_eq!(
            (hp.port, hp.to_string().as_str()),
            (8443, "example.com:8443")
        );
        for bad in [
            "https://example.com",
            "example.com/",
            "example.com:0",
            "",
            "a b",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn rejects_bad_urls() {
        for bad in [
            "example.com",
            "ftp://example.com/",
            "http://",
            "http://user@example.com/",
            "http://example.com:0/",
            "http://example.com:99999/",
            "http://example.com:abc/",
            "http://exa mple.com/",
            "http://bücher.de/",
            "http://example.com/#top",
            "http://[::1/",
            "http://exa%mple.com/",
        ] {
            assert!(Url::try_from(bad.to_owned()).is_err(), "{bad}");
        }
    }
}
