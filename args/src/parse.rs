use std::str::Chars;

fn is_reserved(c: char) -> bool {
    match c {
        '~' | '`' | '#' | '$' | '&' | '*' | '(' | ')' | '\\' | '|' | '[' | ']' | '{' | '}' | ';' | '\'' | '"' | '<' | '>' | '/' | '?' | '!' => true,
        _ => false
    }
}
fn parse_quoted(chars: &mut Chars) -> Result<String, &'static str> {
    let mut build = String::new();
    for c in chars {
        match c {
            '"' => {
                return Ok(build);
            }
            other => {
                build.push(other);
            }
        }
    }
    Err("Failed to find matching '")

}
pub fn parse_unquoted(string: &str) -> Result<Vec<String>,&'static str> {
    let mut vec = Vec::new();
    let mut current_arg = String::new();
    let mut chars = string.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if current_arg.len() != 0 {
                    return Err("Found ' in the middle of arg");
                }
                let quoted = parse_quoted(&mut chars)?;
                vec.push(quoted);
            }
            ' ' => {
                if current_arg.len() != 0 {
                    vec.push(current_arg);
                    current_arg = "".to_string()
                }
            }
            c  if is_reserved(c) => {
                return Err("Reserved character");
            }
            other => {
                current_arg.push(other);
            }
        }
    }
    if current_arg.len() > 0 {
        vec.push(current_arg);
    }
    Ok(vec)
}