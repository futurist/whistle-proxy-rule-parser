/// fork from: https://github.com/hgm-king/prose
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take, take_while1},
    combinator::{map, not},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

pub type MarkdownText = Vec<MarkdownInline>;

#[derive(Clone, Debug, PartialEq)]
pub enum Markdown {
    Line(MarkdownText),
    Codeblock(String, String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MarkdownInline {
    Plaintext(String),
}

pub fn parse_markdown(i: &str) -> IResult<&str, Vec<Markdown>> {
    many1(alt((
        map(parse_code_block, |e| {
            Markdown::Codeblock(e.0.to_string(), e.1.to_string())
        }),
        map(parse_markdown_text, |e| Markdown::Line(e)),
    )))(i)
}

fn parse_plaintext(i: &str) -> IResult<&str, String> {
    map(
        many1(preceded(not(alt((tag("```"), tag("\n")))), take(1u8))),
        |vec| vec.join(""),
    )(i)
}

fn parse_markdown_inline(i: &str) -> IResult<&str, MarkdownInline> {
    alt((map(parse_plaintext, |s| MarkdownInline::Plaintext(s)),))(i)
}

fn parse_markdown_text(i: &str) -> IResult<&str, MarkdownText> {
    terminated(many0(parse_markdown_inline), tag("\n"))(i)
}

fn parse_code_block(i: &str) -> IResult<&str, (String, &str)> {
    tuple((parse_code_block_lang, parse_code_block_body))(i)
}

fn parse_code_block_body(i: &str) -> IResult<&str, &str> {
    delimited(tag("\n"), is_not("```"), tag("```"))(i)
}

fn parse_code_block_lang(i: &str) -> IResult<&str, String> {
    alt((
        preceded(tag("```"), parse_plaintext),
        map(tag("```"), |_| "__UNKNOWN__".to_string()),
    ))(i)
}

/// Break md_arr into (lines, codes)
/// # Examples
/// ```ignore
/// let (rest, md_arr) = parse_markdown(input).unwrap();
/// let (input, codes) = into_parts(md_arr);
/// ```
/// 
pub fn into_parts(md_arr: Vec<Markdown>) -> (String, Vec<(String, String)>) {
    let mut lines = String::new();
    let mut codes = vec![];
    md_arr.iter().for_each(|m| {
        match m {
            Markdown::Line(v) => {
                if v.is_empty() {
                    lines.push('\n');
                    return;
                }
                match &v[0] {
                  MarkdownInline::Plaintext(s) => {
                    lines.push_str(&s);
                    lines.push('\n');
                  }
                  _ => unreachable!(),
              }
            }
            Markdown::Codeblock(name, value) => {
              codes.push((name.to_owned(), value.to_owned()));
            }
        }
    });
    (lines, codes)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_markdown() {
        let input = r#"
# oijsdf
**bold text**
```rust
fn main() {
    println!("Hello, world!");
}
```
**bold**
```js
console.log(1234)
```
`inline code`
"#;
        assert_eq!(
            parse_markdown(input),
            Ok((
                "",
                vec![
                    Markdown::Line(vec![]),
                    Markdown::Line(vec![MarkdownInline::Plaintext("# oijsdf".into())]),
                    Markdown::Line(vec![MarkdownInline::Plaintext("**bold text**".into())]),
                    Markdown::Codeblock(
                        "rust".into(),
                        "fn main() {\n    println!(\"Hello, world!\");\n}\n".into()
                    ),
                    Markdown::Line(vec![]),
                    Markdown::Line(vec![MarkdownInline::Plaintext("**bold**".into())]),
                    Markdown::Codeblock("js".into(), "console.log(1234)\n".into()),
                    Markdown::Line(vec![]),
                    Markdown::Line(vec![MarkdownInline::Plaintext("`inline code`".into())])
                ]
            ))
        );

        assert_eq!(into_parts(parse_markdown(input).unwrap().1), (
            "\n# oijsdf\n**bold text**\n\n**bold**\n\n`inline code`\n".into(),
            vec![("rust".into(), "fn main() {\n    println!(\"Hello, world!\");\n}\n".into()), ("js".into(), "console.log(1234)\n".into())]
        ));

    }
}
