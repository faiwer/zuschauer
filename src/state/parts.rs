use crate::collect::Parts;

pub const NAMES: &str = "host, containers, checks, certificates, services, commands";

/// A comma-separated list of `NAMES`.
pub fn parse_parts(list: &str) -> Result<Parts, String> {
    let mut parts = Parts::NONE;
    for name in list.split(',').map(str::trim) {
        let part = match name {
            "host" => &mut parts.host,
            "containers" => &mut parts.containers,
            "checks" => &mut parts.checks,
            "certificates" => &mut parts.certificates,
            "services" => &mut parts.services,
            "commands" => &mut parts.commands,
            _ => {
                return Err(format!(
                    "--only: unknown part \"{name}\", expected some of {NAMES}"
                ));
            }
        };
        *part = true;
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lists() {
        let parts = parse_parts("host, services").unwrap();
        assert_eq!(
            parts,
            Parts {
                host: true,
                services: true,
                ..Parts::NONE
            }
        );
        assert_eq!(
            parse_parts("host,containers,checks,certificates,services,commands"),
            Ok(Parts::ALL)
        );
        assert!(parse_parts("host,jobs").is_err());
        assert!(parse_parts("").is_err());
    }
}
