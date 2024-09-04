//! A LitExpr (short for LiteralExpression) is similar to a JSON value (or
//! serde_json::Value), with the addition of PathSelection as a possible leaf
//! value, so literal expressions passed to -> methods (via MethodArgs) can
//! incorporate dynamic $variable values in addition to the usual input data and
//! argument values.

use apollo_compiler::collections::IndexMap;
use nom::branch::alt;
use nom::character::complete::char;
use nom::character::complete::one_of;
use nom::combinator::map;
use nom::combinator::opt;
use nom::combinator::recognize;
use nom::multi::many0;
use nom::multi::many1;
use nom::sequence::delimited;
use nom::sequence::pair;
use nom::sequence::preceded;
use nom::sequence::tuple;
use nom::IResult;

use super::helpers::spaces_or_comments;
use super::location::merge_locs;
use super::location::parsed_span;
use super::location::Parsed;
use super::location::Span;
use super::parser::parse_string_literal;
use super::parser::Key;
use super::parser::PathSelection;
use super::ExternalVarPaths;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LitExpr {
    String(String),
    Number(serde_json::Number),
    Bool(bool),
    Null,
    Object(IndexMap<Parsed<Key>, Parsed<LitExpr>>),
    Array(Vec<Parsed<LitExpr>>),
    Path(PathSelection),
}

impl LitExpr {
    // LitExpr      ::= LitPrimitive | LitObject | LitArray | PathSelection
    // LitPrimitive ::= LitString | LitNumber | "true" | "false" | "null"
    pub fn parse(input: Span) -> IResult<Span, Parsed<Self>> {
        tuple((
            spaces_or_comments,
            alt((
                map(parse_string_literal, |s| s.take_as(Self::String)),
                Self::parse_number,
                map(parsed_span("true"), |t| {
                    Parsed::new(Self::Bool(true), t.loc())
                }),
                map(parsed_span("false"), |f| {
                    Parsed::new(Self::Bool(false), f.loc())
                }),
                map(parsed_span("null"), |n| Parsed::new(Self::Null, n.loc())),
                Self::parse_object,
                Self::parse_array,
                map(PathSelection::parse, |path| path.take_as(Self::Path)),
            )),
            spaces_or_comments,
        ))(input)
        .map(|(input, (_, value, _))| (input, value))
    }

    // LitNumber ::= "-"? ([0-9]+ ("." [0-9]*)? | "." [0-9]+)
    fn parse_number(input: Span) -> IResult<Span, Parsed<Self>> {
        let (suffix, (neg, _spaces, num)) = delimited(
            spaces_or_comments,
            tuple((
                opt(char('-')),
                spaces_or_comments,
                alt((
                    recognize(pair(
                        many1(one_of("0123456789")),
                        opt(preceded(char('.'), many0(one_of("0123456789")))),
                    )),
                    recognize(pair(tag("."), many1(one_of("0123456789")))),
                )),
            )),
            spaces_or_comments,
        )(input)?;

        let mut number = String::new();
        if let Some('-') = neg {
            number.push('-');
        }
        if num.starts_with('.') {
            // The serde_json::Number::parse method requires a leading digit
            // before the decimal point.
            number.push('0');
        }
        number.push_str(*num.fragment());
        if num.ends_with('.') {
            // The serde_json::Number::parse method requires a trailing digit
            // after the decimal point.
            number.push('0');
        }

        if let Ok(lit_number) = number.parse().map(Self::Number) {
            let loc = Some((
                num.location_offset(),
                num.location_offset() + num.fragment().len(),
            ));

            let loc = if let Some(neg) = neg {
                merge_locs(neg.loc(), loc)
            } else {
                loc
            };

            Ok((suffix, Parsed::new(lit_number, loc)))
        } else {
            Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::IsNot,
            )))
        }
    }

    // LitObject ::= "{" (LitProperty ("," LitProperty)* ","?)? "}"
    fn parse_object(input: Span) -> IResult<Span, Parsed<Self>> {
        tuple((
            spaces_or_comments,
            parsed_span("{"),
            spaces_or_comments,
            map(
                opt(tuple((
                    Self::parse_property,
                    many0(preceded(char(','), Self::parse_property)),
                    opt(char(',')),
                ))),
                |properties| {
                    let mut output = IndexMap::default();
                    if let Some(((first_key, first_value), rest, _trailing_comma)) = properties {
                        output.insert(first_key, first_value);
                        for (key, value) in rest {
                            output.insert(key, value);
                        }
                    }
                    Self::Object(output)
                },
            ),
            spaces_or_comments,
            parsed_span("}"),
            spaces_or_comments,
        ))(input)
        .map(|(input, (_, open_brace, _, output, _, close_brace, _))| {
            let loc = merge_locs(open_brace.loc(), close_brace.loc());
            (input, Parsed::new(output, loc))
        })
    }

    // LitProperty ::= Key ":" LitExpr
    fn parse_property(input: Span) -> IResult<Span, (Parsed<Key>, Parsed<Self>)> {
        tuple((Key::parse, char(':'), Self::parse))(input)
            .map(|(input, (key, _, value))| (input, (key, value)))
    }

    // LitArray ::= "[" (LitExpr ("," LitExpr)* ","?)? "]"
    fn parse_array(input: Span) -> IResult<Span, Parsed<Self>> {
        tuple((
            spaces_or_comments,
            parsed_span("["),
            spaces_or_comments,
            map(
                opt(tuple((
                    Self::parse,
                    many0(preceded(char(','), Self::parse)),
                    opt(char(',')),
                ))),
                |elements| {
                    let mut output = vec![];
                    if let Some((first, rest, _trailing_comma)) = elements {
                        output.push(first);
                        output.extend(rest);
                    }
                    Self::Array(output)
                },
            ),
            spaces_or_comments,
            parsed_span("]"),
            spaces_or_comments,
        ))(input)
        .map(
            |(input, (_, open_bracket, _, output, _, close_bracket, _))| {
                let loc = merge_locs(open_bracket.loc(), close_bracket.loc());
                (input, Parsed::new(output, loc))
            },
        )
    }

    pub(super) fn into_parsed(self) -> Parsed<Self> {
        Parsed::new(self, None)
    }

    pub(super) fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }
}

impl ExternalVarPaths for LitExpr {
    fn external_var_paths(&self) -> Vec<&PathSelection> {
        let mut paths = vec![];
        match self {
            Self::String(_) | Self::Number(_) | Self::Bool(_) | Self::Null => {}
            Self::Object(map) => {
                for value in map.values() {
                    paths.extend(value.external_var_paths());
                }
            }
            Self::Array(vec) => {
                for value in vec {
                    paths.extend(value.external_var_paths());
                }
            }
            Self::Path(path) => {
                paths.extend(path.external_var_paths());
            }
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::super::known_var::KnownVariable;
    use super::super::location::strip_loc::StripLoc;
    use super::*;
    use crate::sources::connect::json_selection::PathList;

    fn check_parse(input: &str, expected: LitExpr) {
        match LitExpr::parse(Span::new(input)) {
            Ok((remainder, parsed)) => {
                assert_eq!(*remainder.fragment(), "");
                assert_eq!(parsed.strip_loc(), Parsed::new(expected, None));
            }
            Err(e) => panic!("Failed to parse '{}': {:?}", input, e),
        };
    }

    #[test]
    fn test_lit_expr_parse_primitives() {
        check_parse("'hello'", LitExpr::String("hello".to_string()));
        check_parse("\"hello\"", LitExpr::String("hello".to_string()));
        check_parse(" 'hello' ", LitExpr::String("hello".to_string()));
        check_parse(" \"hello\" ", LitExpr::String("hello".to_string()));

        check_parse("123", LitExpr::Number(serde_json::Number::from(123)));
        check_parse("-123", LitExpr::Number(serde_json::Number::from(-123)));
        check_parse(" - 123 ", LitExpr::Number(serde_json::Number::from(-123)));
        check_parse(
            "123.456",
            LitExpr::Number(serde_json::Number::from_f64(123.456).unwrap()),
        );
        check_parse(
            ".456",
            LitExpr::Number(serde_json::Number::from_f64(0.456).unwrap()),
        );
        check_parse(
            "-.456",
            LitExpr::Number(serde_json::Number::from_f64(-0.456).unwrap()),
        );
        check_parse(
            "123.",
            LitExpr::Number(serde_json::Number::from_f64(123.0).unwrap()),
        );
        check_parse(
            "-123.",
            LitExpr::Number(serde_json::Number::from_f64(-123.0).unwrap()),
        );

        check_parse("true", LitExpr::Bool(true));
        check_parse(" true ", LitExpr::Bool(true));
        check_parse("false", LitExpr::Bool(false));
        check_parse(" false ", LitExpr::Bool(false));
        check_parse("null", LitExpr::Null);
        check_parse(" null ", LitExpr::Null);
    }

    #[test]
    fn test_lit_expr_parse_objects() {
        check_parse(
            "{a: 1}",
            LitExpr::Object({
                let mut map = IndexMap::default();
                map.insert(
                    Key::field("a").into_parsed(),
                    LitExpr::Number(serde_json::Number::from(1)).into_parsed(),
                );
                map
            }),
        );

        check_parse(
            "{'a': 1}",
            LitExpr::Object({
                let mut map = IndexMap::default();
                map.insert(
                    Key::quoted("a").into_parsed(),
                    LitExpr::Number(serde_json::Number::from(1)).into_parsed(),
                );
                map
            }),
        );

        {
            fn make_expected(a_key: Key, b_key: Key) -> LitExpr {
                let mut map = IndexMap::default();
                map.insert(
                    a_key.into_parsed(),
                    LitExpr::Number(serde_json::Number::from(1)).into_parsed(),
                );
                map.insert(
                    b_key.into_parsed(),
                    LitExpr::Number(serde_json::Number::from(2)).into_parsed(),
                );
                LitExpr::Object(map)
            }
            check_parse(
                "{'a': 1, 'b': 2}",
                make_expected(Key::quoted("a"), Key::quoted("b")),
            );
            check_parse(
                "{ a : 1, 'b': 2}",
                make_expected(Key::field("a"), Key::quoted("b")),
            );
            check_parse(
                "{ a : 1, b: 2}",
                make_expected(Key::field("a"), Key::field("b")),
            );
            check_parse(
                "{ \"a\" : 1, \"b\": 2 }",
                make_expected(Key::quoted("a"), Key::quoted("b")),
            );
            check_parse(
                "{ \"a\" : 1, b: 2 }",
                make_expected(Key::quoted("a"), Key::field("b")),
            );
            check_parse(
                "{ a : 1, \"b\": 2 }",
                make_expected(Key::field("a"), Key::quoted("b")),
            );
        }
    }

    #[test]
    fn test_lit_expr_parse_arrays() {
        check_parse(
            "[1, 2]",
            LitExpr::Array(vec![
                Parsed::new(LitExpr::Number(serde_json::Number::from(1)), None),
                Parsed::new(LitExpr::Number(serde_json::Number::from(2)), None),
            ]),
        );

        check_parse(
            "[1, true, 'three']",
            LitExpr::Array(vec![
                Parsed::new(LitExpr::Number(serde_json::Number::from(1)), None),
                Parsed::new(LitExpr::Bool(true), None),
                Parsed::new(LitExpr::String("three".to_string()), None),
            ]),
        );
    }

    #[test]
    fn test_lit_expr_parse_paths() {
        {
            let expected = LitExpr::Path(PathSelection {
                path: PathList::Key(
                    Key::field("a").into_parsed(),
                    PathList::Key(
                        Key::field("b").into_parsed(),
                        PathList::Key(Key::field("c").into_parsed(), PathList::Empty.into_parsed())
                            .into_parsed(),
                    )
                    .into_parsed(),
                )
                .into_parsed(),
            });

            check_parse("a.b.c", expected.clone());
            check_parse(" a . b . c ", expected.clone());
        }

        {
            let expected = LitExpr::Path(PathSelection {
                path: PathList::Key(
                    Key::field("data").into_parsed(),
                    PathList::Empty.into_parsed(),
                )
                .into_parsed(),
            });
            check_parse(".data", expected.clone());
            check_parse(" . data ", expected.clone());
        }

        {
            let expected = LitExpr::Array(vec![
                LitExpr::Path(PathSelection {
                    path: PathList::Key(
                        Key::field("a").into_parsed(),
                        PathList::Empty.into_parsed(),
                    )
                    .into_parsed(),
                })
                .into_parsed(),
                LitExpr::Path(PathSelection {
                    path: PathList::Key(
                        Key::field("b").into_parsed(),
                        PathList::Key(Key::field("c").into_parsed(), PathList::Empty.into_parsed())
                            .into_parsed(),
                    )
                    .into_parsed(),
                })
                .into_parsed(),
                LitExpr::Path(PathSelection {
                    path: PathList::Key(
                        Key::field("d").into_parsed(),
                        PathList::Key(
                            Key::field("e").into_parsed(),
                            PathList::Key(
                                Key::field("f").into_parsed(),
                                PathList::Empty.into_parsed(),
                            )
                            .into_parsed(),
                        )
                        .into_parsed(),
                    )
                    .into_parsed(),
                })
                .into_parsed(),
            ]);

            check_parse("[.a, b.c, .d.e.f]", expected.clone());
            check_parse("[.a, b.c, .d.e.f,]", expected.clone());
            check_parse("[ . a , b . c , . d . e . f ]", expected.clone());
            check_parse("[ . a , b . c , . d . e . f , ]", expected.clone());
            check_parse(
                r#"[
                .a,
                b.c,
                .d.e.f,
            ]"#,
                expected.clone(),
            );
            check_parse(
                r#"[
                . a ,
                . b . c ,
                d . e . f ,
            ]"#,
                expected.clone(),
            );
        }

        {
            let expected = LitExpr::Object({
                let mut map = IndexMap::default();
                map.insert(
                    Key::field("a").into_parsed(),
                    LitExpr::Path(PathSelection {
                        path: PathList::Var(
                            KnownVariable::Args.into_parsed(),
                            PathList::Key(
                                Key::field("a").into_parsed(),
                                PathList::Empty.into_parsed(),
                            )
                            .into_parsed(),
                        )
                        .into_parsed(),
                    })
                    .into_parsed(),
                );
                map.insert(
                    Key::field("b").into_parsed(),
                    LitExpr::Path(PathSelection {
                        path: PathList::Var(
                            KnownVariable::This.into_parsed(),
                            PathList::Key(
                                Key::field("b").into_parsed(),
                                PathList::Empty.into_parsed(),
                            )
                            .into_parsed(),
                        )
                        .into_parsed(),
                    })
                    .into_parsed(),
                );
                map
            });

            check_parse(
                r#"{
                a: $args.a,
                b: $this.b,
            }"#,
                expected.clone(),
            );

            check_parse(
                r#"{
                b: $this.b,
                a: $args.a,
            }"#,
                expected.clone(),
            );

            check_parse(
                r#" {
                a : $args . a ,
                b : $this . b
            ,} "#,
                expected.clone(),
            );
        }
    }
}
