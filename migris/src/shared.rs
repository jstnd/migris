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

/// Sanitizes the given string to be safe as a file name.
pub fn sanitize_file_name(s: &str) -> String {
    const REPLACE_CHAR: char = '-';
    let mut prev_replace = false;

    s.chars()
        .filter_map(|c| match c {
            c if c.is_alphanumeric() => {
                prev_replace = false;
                Some(c)
            }
            _ => {
                if prev_replace {
                    None
                } else {
                    prev_replace = true;
                    Some(REPLACE_CHAR)
                }
            }
        })
        .collect()
}
