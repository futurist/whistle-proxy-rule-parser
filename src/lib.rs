use nom::character::is_space;
use nom::combinator::all_consuming;
use nom::error::{context, ErrorKind, ParseError};
use nom::Err::Error;
use nom::character::complete::{none_of, space0};
use nom::multi::separated_list0;
use nom::Parser;
use nom::{branch::alt, bytes::complete::is_not, multi::many0, sequence::delimited};
use nom::{
    bytes::complete::{tag, take_until, take_till1, take_while, take_while1},
    character::complete::{multispace0, multispace1, space1, char as char1},
    combinator::{opt, map},
    sequence::{preceded, terminated, tuple},
    IResult,
};

pub mod markdown_values;

#[derive(Debug, Clone)]
pub struct Uri {
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub query: String,
}

#[derive(Debug, Clone)]
pub enum OpValue {
    Inline(String),
    Value(String),
    Raw(String),
    TemplateString(TemplateString),
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub value: OpValue,
}

#[derive(Debug, Clone)]
pub enum TemplatePart {
    RawString(String),
    Value(String),
}

#[derive(Debug, Clone)]
pub struct TemplateString {
    pub parts: Vec<TemplatePart>,
}

#[derive(Debug, Clone)]
pub struct ProxyRule {
  pub source: Uri,
  pub target: Uri,
  pub rules: Vec<Rule>,
}

#[derive(Debug, PartialEq)]
pub enum CustomError<I> {
  MyError,
  Nom(I, ErrorKind),
}

impl<I> ParseError<I> for CustomError<I> {
  fn from_error_kind(input: I, kind: ErrorKind) -> Self {
    CustomError::Nom(input, kind)
  }

  fn append(_: I, _: ErrorKind, other: Self) -> Self {
    other
  }
}

pub fn error_from_str(_input: &str) -> IResult<&str, &str, CustomError<&str>> {
  Err(Error(CustomError::MyError))
}


fn whitespace<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
  take_while1(|c: char| c.is_whitespace())(i)
}

fn not_space(s: &str) -> IResult<&str, &str> {
  take_while1(|c:char| !c.is_whitespace())(s)
}

fn parse_escaped(input: &str) -> IResult<&str, TemplatePart> {
    let (input, _) = tag("\\")(input)?;
    let (input, escaped) = none_of("\\")(input)?;
    Ok((input, TemplatePart::RawString(escaped.to_string())))
}

fn parse_template_string(input: &str) -> IResult<&str, TemplateString> {
    let original_input = input;
    let (mut input, bracket) = opt(char1('('))(input)?;
    if bracket.is_some() {
        input = input.strip_suffix(")").expect(&format!("{original_input} format is wrong"));
    }
    let (input, parts) = many0(
        nom::branch::alt((
            parse_escaped,
            map(preceded(tag("${"), terminated(take_until("}"), tag("}"))), |s: &str| TemplatePart::Value(s.to_string())),
            map(take_until("${"), |s: &str| TemplatePart::RawString(s.to_string())),
        )),
    )(input)?;

    Ok((input, TemplateString { parts }))
}

fn parse_uri(input: &str) -> IResult<&str, Uri> {
    let (input, (scheme, host, path, query)) = tuple((
        opt(terminated(
            take_while1(|c: char| c.is_alphanumeric()),
            tag("://"),
        )),
        opt(take_while1(|c: char| c != '/')),
        take_while(|c: char| c != '?'),
        take_while(|c: char| !c.is_whitespace()),
    ))(input)?;

    Ok((
        input,
        Uri {
            scheme: scheme.unwrap_or_default().to_string(),
            host: host.unwrap_or_default().to_string(),
            path: path.to_string(),
            query: query.to_string(),
        },
    ))
}

fn parse_rule_value(input: &str) -> IResult<&str, OpValue> {
    let (input, opval) = alt((
        map(delimited(char1('`'), take_while(|c: char|c != ' ' && c != '\t' && c != '`'), char1('`')), |s:&str| OpValue::TemplateString(parse_template_string(s).unwrap().1)),
        map(delimited(char1('('), take_while(|c: char|c != ' ' && c != '\t' && c != ')'), char1(')')), |s:&str| OpValue::Inline(s.to_string())),
        map(delimited(char1('{'), take_while(|c: char|c != ' ' && c != '\t' && c != '}'), char1('}')), |s:&str| OpValue::Value(s.to_string())),
        map(take_while(|c:char| !is_space(c as u8) ), |s: &str| OpValue::Raw(s.to_string())),
    ))(input)?;

    Ok((
        input,
        opval,
    ))
}

fn parse_rule(input: &str) -> IResult<&str, Rule> {
    let (input, (name, value)) = tuple((
        terminated(take_while1(|c: char| c.is_alphanumeric()), tag("://")),
        map(take_while(|c: char| !c.is_whitespace()), |s:&str| parse_rule_value(s)),
    ))(input)?;

    let (_, value) = value?;

    Ok((
        input,
        Rule {
            name: name.to_string(),
            value,
        },
    ))
}

fn get_part(input: &str) -> IResult<&str, &str> {
    preceded(multispace0, take_till1(|c: char| c.is_whitespace()))(input)
}

fn get_rules(input: &str) -> IResult<&str, Vec<Rule>> {
  let (rest, rules) = preceded(whitespace, separated_list0(whitespace, map(not_space, |s:&str|  {
    parse_rule(s).unwrap().1
}))).parse(input)?;

  Ok((
    rest,
    rules,
  ))
}

// The error handler will trigger a 'static str reference, solution is here:
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=2de79a2b85310e11e915c674b28a9246
// Issue: https://github.com/rust-bakery/nom/issues/1571
pub fn parse_proxy_rule(input: &str) -> IResult<&str, ProxyRule> {
    let (rest, source) = map(get_part, all_consuming(parse_uri))(input)?;
    let source = source?.1;
    // println!("source: {:#?}", source);

    let (rest, target) = map(get_part, all_consuming(parse_uri))(rest)?;
    let target = target?.1;
    // println!("target: {:#?}", target);

    let (rest, rules) = if rest.trim().is_empty() {
      (rest, vec![])
    } else {
      get_rules(rest).unwrap()
    };

    Ok((
      rest,
      ProxyRule {
        source,
        target,
        rules,
      }
    ))
}
