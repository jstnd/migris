/// The default port used by MySQL.
///
/// https://dev.mysql.com/doc/mysql-port-reference/en/mysql-port-reference-tables.html
pub const DEFAULT_MYSQL_PORT: u16 = 3306;

/// Returns the given bytes converted into hex format.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        _ = write!(&mut s, "{:02x}", b);
    }
    s
}
