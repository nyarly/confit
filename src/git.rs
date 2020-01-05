pub mod parse;
pub mod exec {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_accepts_process_output() {
        let stdout: Vec<u8> = include_str!("git/parse/testdata/mezzo-ls-remote").into();
        parse::ls_remote::parse(&String::from_utf8(stdout).unwrap()).unwrap();
    }
}
