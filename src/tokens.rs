/// Determines if byte is a token char
/// !, #, $, %, &, ', * +, -, ., ^, _, `, |, ~, digits, alphanumeric
pub fn is_token(b: u8) -> bool {
    b > b'_' && b < 0x7f
}

// ASCII codes to accept as part of URI strings
// A-Z a-z 0-9 !#$%&'*+-._();:@=,/?[]~^
pub fn is_uri_token(ch: u8) -> bool {
    match ch {
        0..=b' ' => false,
        b'<' | b'>' => false,
        b'!'..=b'~' => true,
        0x7f.. => false,
    }
}

// ASCII codes to accept as part of header names
pub fn is_header_name_token(ch: u8) -> bool {
    match ch {
        b'!' => true,
        b'#'..=b'/' => true, /* #, $, %, &, ' */
        b'|' | b'~' | b'^' | b'_' | b'`' => true,
        b'0'..=b'9' => true,
        b'A'..=b'Z' => true,
        b'a'..=b'z' => true,
        _ => false,
    }
}

// ASCII codes to accept as part of header values
pub fn is_header_value_token(ch: u8) -> bool {
    match ch {
        0x9 => true,
        0x7f => false,
        b' '..=0xff => true,
        _ => false,
    }
}
