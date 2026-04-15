use crate::Entity;

const CHAR_ASTERISK: char = '*';
const CHAR_BACKSLASH: char = '\\';
const CHAR_BACKTICK: char = '`';
const CHAR_DASH: char = '-';
const CHAR_HASH: char = '#';
const CHAR_NEWLINE: char = '\n';
const CHAR_SLASH: char = '/';
const CHAR_QUOTE_DOUBLE: char = '"';
const CHAR_QUOTE_SINGLE: char = '\'';

#[derive(Debug, PartialEq, Eq)]
pub enum SqlStatementKind {
    Command,
    Query,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SqlStatement {
    pub kind: SqlStatementKind,
    pub sql: String,
}

pub fn select_all(entity: &Entity) -> String {
    format!("SELECT * FROM `{}`.`{}`", entity.schema, entity.name)
}

pub fn split(sql: &str) -> Vec<SqlStatement> {
    enum SplitState {
        Normal,
        Backtick,
        BlockComment,
        DoubleQuote,
        LineComment,
        SingleQuote,
    }

    let mut statements = Vec::new();
    let mut state = SplitState::Normal;
    let chars: Vec<char> = sql.chars().collect();

    let mut idx = 0;
    let mut start_idx = 0;

    while idx < chars.len() {
        let char = chars[idx];

        match state {
            SplitState::Normal => match char {
                CHAR_BACKTICK => state = SplitState::Backtick,
                CHAR_DASH => {
                    // TODO: handle double-dash line comments
                }
                CHAR_HASH => state = SplitState::LineComment,
                CHAR_SLASH => {
                    if chars.get(idx + 1) == Some(&CHAR_ASTERISK) {
                        state = SplitState::BlockComment;

                        // Skip the opening asterisk for the comment.
                        idx += 1;
                    }
                }
                CHAR_QUOTE_DOUBLE => state = SplitState::DoubleQuote,
                CHAR_QUOTE_SINGLE => state = SplitState::SingleQuote,
                _ => {
                    // TODO: handle DELIMITER command

                    if char == ';' {
                        let sql = sql[start_idx..idx].trim().to_owned();

                        // Skip empty queries.
                        if !sql.is_empty() {
                            statements.push(SqlStatement {
                                kind: SqlStatementKind::Query,
                                sql,
                            });
                        }

                        // Move the start index to the next character after the delimiter.
                        start_idx = idx + 1;
                    }
                }
            },
            SplitState::Backtick => {
                if char == CHAR_BACKTICK {
                    state = SplitState::Normal;
                }
            }
            SplitState::BlockComment => {
                if char == CHAR_ASTERISK && chars.get(idx + 1) == Some(&CHAR_SLASH) {
                    state = SplitState::Normal;

                    // Skip the closing slash for the comment.
                    idx += 1;
                }
            }
            SplitState::DoubleQuote => match char {
                CHAR_BACKSLASH => {
                    // Skip the escaped character after the backslash.
                    idx += 1;
                }
                CHAR_QUOTE_DOUBLE => {
                    state = SplitState::Normal;
                }
                _ => {}
            },
            SplitState::LineComment => {
                if char == CHAR_NEWLINE {
                    state = SplitState::Normal;
                }
            }
            SplitState::SingleQuote => match char {
                CHAR_BACKSLASH => {
                    // Skip the escaped character after the backslash.
                    idx += 1;
                }
                CHAR_QUOTE_SINGLE => {
                    state = SplitState::Normal;
                }
                _ => {}
            },
        }

        // Move to the next character.
        idx += 1;
    }

    // Handle trailing statement without end delimiter.
    if start_idx < chars.len() {
        let sql = sql[start_idx..].trim().to_owned();

        // Skip empty queries.
        if !sql.is_empty() {
            statements.push(SqlStatement {
                kind: SqlStatementKind::Query,
                sql,
            });
        }
    }

    statements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_simple_query() {
        assert_eq!(
            split("SELECT * FROM test"),
            [SqlStatement {
                kind: SqlStatementKind::Query,
                sql: "SELECT * FROM test".to_owned()
            }]
        );

        assert_eq!(
            split("SELECT * FROM test; SELECT * FROM test2 WHERE 1 = 1"),
            [
                SqlStatement {
                    kind: SqlStatementKind::Query,
                    sql: "SELECT * FROM test".to_owned(),
                },
                SqlStatement {
                    kind: SqlStatementKind::Query,
                    sql: "SELECT * FROM test2 WHERE 1 = 1".to_owned()
                }
            ]
        );

        assert_eq!(
            split("SELECT 'test;single;quote'; SELECT \"test;double;quote;\""),
            [
                SqlStatement {
                    kind: SqlStatementKind::Query,
                    sql: "SELECT 'test;single;quote'".to_owned()
                },
                SqlStatement {
                    kind: SqlStatementKind::Query,
                    sql: "SELECT \"test;double;quote;\"".to_owned()
                }
            ]
        )
    }

    fn split_with_comments() {}
    fn split_with_delimiter() {}
}
