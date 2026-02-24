use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Text(String),
    Code(String),
    Expression(String),
    Directive(String),
    Declaration(String),
}

#[derive(Debug, Clone, Default)]
pub struct ParsedTemplate {
    pub tokens: Vec<Token>,
    pub directives: Vec<String>,
    pub declarations: Vec<String>,
}

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    pub fn parse(&self, input: &str) -> Result<ParsedTemplate, ParseError> {
        let mut tokens = Vec::new();
        let mut directives = Vec::new();
        let mut declarations = Vec::new();
        let mut chars = input.chars().peekable();
        let mut text_buf = String::new();

        while let Some(ch) = chars.next() {
            if ch == '<' {
                if let Some(&'<') = chars.peek() {
                    text_buf.push('<');
                    chars.next();
                    continue;
                }
                if let Some(&'%') = chars.peek() {
                    chars.next();

                    if !text_buf.is_empty() {
                        tokens.push(Token::Text(text_buf.clone()));
                        text_buf.clear();
                    }

                    let tag_type = match chars.peek() {
                        Some(&'=') => {
                            chars.next();
                            TagType::Expression
                        }
                        Some(&'@') => {
                            chars.next();
                            TagType::Directive
                        }
                        Some(&'!') => {
                            chars.next();
                            TagType::Declaration
                        }
                        _ => TagType::Code,
                    };

                    let mut code_buf = String::new();

                    loop {
                        match chars.next() {
                            None => {
                                return Err(ParseError::UnclosedTag);
                            }
                            Some('%') => {
                                if let Some(&'>') = chars.peek() {
                                    chars.next();
                                    break;
                                }
                                code_buf.push('%');
                            }
                            Some(c) => {
                                code_buf.push(c);
                            }
                        }
                    }

                    let content = code_buf.trim().to_string();

                    match tag_type {
                        TagType::Expression => {
                            tokens.push(Token::Expression(content));
                        }
                        TagType::Code => {
                            tokens.push(Token::Code(content));
                        }
                        TagType::Directive => {
                            directives.push(content.clone());
                            tokens.push(Token::Directive(content));
                        }
                        TagType::Declaration => {
                            declarations.push(content.clone());
                            tokens.push(Token::Declaration(content));
                        }
                    }
                } else {
                    text_buf.push(ch);
                }
            } else {
                text_buf.push(ch);
            }
        }

        if !text_buf.is_empty() {
            tokens.push(Token::Text(text_buf));
        }

        Ok(ParsedTemplate {
            tokens,
            directives,
            declarations,
        })
    }
}

enum TagType {
    Code,
    Expression,
    Directive,
    Declaration,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum ParseError {
    UnclosedTag,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnclosedTag => write!(f, "Unclosed <% %> tag"),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text() {
        let parser = Parser::new();
        let result = parser.parse("Hello World").unwrap();
        assert_eq!(result.tokens, vec![Token::Text("Hello World".to_string())]);
    }

    #[test]
    fn test_parse_expression() {
        let parser = Parser::new();
        let result = parser.parse("<%= name %>").unwrap();
        assert_eq!(result.tokens, vec![Token::Expression("name".to_string())]);
    }

    #[test]
    fn test_parse_directive() {
        let parser = Parser::new();
        let result = parser.parse("<%@ database mysql=\"test\" %>").unwrap();
        assert_eq!(result.directives, vec!["database mysql=\"test\""]);
    }

    #[test]
    fn test_parse_declaration() {
        let parser = Parser::new();
        let result = parser.parse("<%! static mut COUNT: i32 = 0; %>").unwrap();
        assert_eq!(result.declarations, vec!["static mut COUNT: i32 = 0;"]);
    }
}
