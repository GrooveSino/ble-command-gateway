pub const NULL_DEVICE_SERIAL: &str = "null";
pub const DEFAULT_ALIAS: &str = "bleinit";

pub fn build_device_name_from_mac(base_prefix: &str, address: Option<[u8; 6]>) -> String {
    let serial = address
        .filter(is_usable_mac_address)
        .map(serial_from_mac)
        .unwrap_or_else(|| NULL_DEVICE_SERIAL.to_string());
    compose_device_name(base_prefix, None, &serial)
}

pub fn compose_device_name(base_prefix: &str, alias: Option<&str>, serial: &str) -> String {
    if serial == NULL_DEVICE_SERIAL {
        return format!("{base_prefix}-{serial}");
    }
    let alias = match alias {
        Some(value) if !value.is_empty() && value != DEFAULT_ALIAS => value,
        _ => DEFAULT_ALIAS,
    };
    format!("{base_prefix}-{alias}-{serial}")
}

pub fn advertised_alias(alias: Option<&str>) -> Option<&str> {
    alias.filter(|value| !value.is_empty() && *value != DEFAULT_ALIAS)
}

pub fn serial_from_mac(address: [u8; 6]) -> String {
    format!("{:02x}{:02x}{:02x}", address[3], address[4], address[5])
}

pub fn is_usable_mac_address(address: &[u8; 6]) -> bool {
    *address != [0; 6] && *address != [0xff; 6]
}

#[cfg(test)]
mod tests {
    use super::{
        advertised_alias, build_device_name_from_mac, compose_device_name, is_usable_mac_address,
        serial_from_mac, DEFAULT_ALIAS,
    };

    #[test]
    fn device_name_uses_default_alias_and_last_six_mac_digits() {
        let address = [0xdc, 0xa6, 0x32, 0x12, 0xab, 0xcd];

        assert_eq!(serial_from_mac(address), "12abcd");
        assert_eq!(
            build_device_name_from_mac("yundrone", Some(address)),
            "yundrone-bleinit-12abcd"
        );
        assert_eq!(
            build_device_name_from_mac("edge", Some(address)),
            "edge-bleinit-12abcd"
        );
    }

    #[test]
    fn compose_inserts_user_alias_between_prefix_and_serial() {
        assert_eq!(
            compose_device_name("yundrone", Some("lab1"), "12abcd"),
            "yundrone-lab1-12abcd"
        );
        assert_eq!(
            compose_device_name("yundrone", None, "12abcd"),
            "yundrone-bleinit-12abcd"
        );
        assert_eq!(
            compose_device_name("yundrone", Some(DEFAULT_ALIAS), "12abcd"),
            "yundrone-bleinit-12abcd"
        );
    }

    #[test]
    fn missing_or_invalid_mac_uses_null_suffix() {
        assert_eq!(
            build_device_name_from_mac("yundrone", None),
            "yundrone-null"
        );
        assert_eq!(
            build_device_name_from_mac("yundrone", Some([0; 6])),
            "yundrone-null"
        );
        assert_eq!(
            compose_device_name("yundrone", Some("lab1"), "null"),
            "yundrone-null"
        );
        assert!(!is_usable_mac_address(&[0; 6]));
        assert!(!is_usable_mac_address(&[0xff; 6]));
    }

    #[test]
    fn advertised_alias_omits_default_placeholder() {
        assert_eq!(advertised_alias(Some("lab1")), Some("lab1"));
        assert_eq!(advertised_alias(Some(DEFAULT_ALIAS)), None);
        assert_eq!(advertised_alias(None), None);
    }
}
