use self::SeparatorCategory::*;
use slice_group_by::StrGroupBy;
use std::iter::Peekable;

pub fn is_cjk(c: char) -> bool {
    (c >= '\u{2e80}' && c <= '\u{2eff}')
        || (c >= '\u{2f00}' && c <= '\u{2fdf}')
        || (c >= '\u{3040}' && c <= '\u{309f}')
        || (c >= '\u{30a0}' && c <= '\u{30ff}')
        || (c >= '\u{3100}' && c <= '\u{312f}')
        || (c >= '\u{3200}' && c <= '\u{32ff}')
        || (c >= '\u{3400}' && c <= '\u{4dbf}')
        || (c >= '\u{4e00}' && c <= '\u{9fff}')
        || (c >= '\u{f900}' && c <= '\u{faff}')
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SeparatorCategory {
    Soft,
    Hard,
}

impl SeparatorCategory {
    fn merge(self, other: SeparatorCategory) -> SeparatorCategory {
        if let (Soft, Soft) = (self, other) {
            Soft
        } else {
            Hard
        }
    }

    fn to_usize(self) -> usize {
        match self {
            Soft => 1,
            Hard => 8,
        }
    }
}

fn is_separator(c: char) -> bool {
    classify_separator(c).is_some()
}

fn classify_separator(c: char) -> Option<SeparatorCategory> {
    match c {
        ' ' | '-' | '_' | '\'' | ':' | '"' => Some(Soft),
        '.' | ';' | ',' | '!' | '?' | '(' | ')' => Some(Hard),
        _ => None,
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CharCategory {
    Separator(SeparatorCategory),
    Cjk,
    Other,
}

fn classify_char(c: char) -> CharCategory {
    if let Some(category) = classify_separator(c) {
        CharCategory::Separator(category)
    } else if is_cjk(c) {
        CharCategory::Cjk
    } else {
        CharCategory::Other
    }
}

fn is_str_word(s: &str) -> bool {
    !s.chars().any(is_separator)
}

fn same_group_category(a: char, b: char) -> bool {
    match (classify_char(a), classify_char(b)) {
        (CharCategory::Cjk, _) | (_, CharCategory::Cjk) => false,
        (CharCategory::Separator(_), CharCategory::Separator(_)) => true,
        (a, b) => a == b,
    }
}

// fold the number of chars along with the index position
fn chars_count_index((n, _): (usize, usize), (i, c): (usize, char)) -> (usize, usize) {
    (n + 1, i + c.len_utf8())
}

pub fn split_query_string(query: &str) -> impl Iterator<Item = &str> {
    Tokenizer::new(query).map(|t| t.word)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub word: &'a str,
    pub word_index: usize,
    pub char_index: usize,
}

pub struct Tokenizer<'a> {
    inner: &'a str,
    word_index: usize,
    char_index: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(string: &str) -> Tokenizer {
        // skip every separator and set `char_index`
        // to the number of char trimmed
        let (count, index) = string
            .char_indices()
            .take_while(|(_, c)| is_separator(*c))
            .fold((0, 0), chars_count_index);

        Tokenizer {
            inner: &string[index..],
            word_index: 0,
            char_index: count,
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.inner.linear_group_by(same_group_category).peekable();

        while let (Some(string), next_string) = (iter.next(), iter.peek()) {
            let (count, index) = string.char_indices().fold((0, 0), chars_count_index);

            if !is_str_word(string) {
                self.word_index += string
                    .chars()
                    .filter_map(classify_separator)
                    .fold(Soft, |a, x| a.merge(x))
                    .to_usize();
                self.char_index += count;
                self.inner = &self.inner[index..];
                continue;
            }

            let token = Token {
                word: string,
                word_index: self.word_index,
                char_index: self.char_index,
            };

            if next_string.filter(|s| is_str_word(s)).is_some() {
                self.word_index += 1;
            }

            self.char_index += count;
            self.inner = &self.inner[index..];

            return Some(token);
        }

        self.inner = "";
        None
    }
}

pub struct SeqTokenizer<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    inner: I,
    current: Option<Peekable<Tokenizer<'a>>>,
    word_offset: usize,
    char_offset: usize,
}

impl<'a, I> SeqTokenizer<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    pub fn new(mut iter: I) -> SeqTokenizer<'a, I> {
        let current = iter.next().map(|s| Tokenizer::new(s).peekable());
        SeqTokenizer {
            inner: iter,
            current,
            word_offset: 0,
            char_offset: 0,
        }
    }
}

impl<'a, I> Iterator for SeqTokenizer<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.current {
            Some(current) => {
                match current.next() {
                    Some(token) => {
                        // we must apply the word and char offsets
                        // to the token before returning it
                        let token = Token {
                            word: token.word,
                            word_index: token.word_index + self.word_offset,
                            char_index: token.char_index + self.char_offset,
                        };

                        // if this is the last iteration on this text
                        // we must save the offsets for next texts
                        if current.peek().is_none() {
                            let hard_space = SeparatorCategory::Hard.to_usize();
                            self.word_offset = token.word_index + hard_space;
                            self.char_offset = token.char_index + hard_space;
                        }

                        Some(token)
                    }
                    None => {
                        // no more words in this text we must
                        // start tokenizing the next text
                        self.current = self.inner.next().map(|s| Tokenizer::new(s).peekable());
                        self.next()
                    }
                }
            }
            // no more texts available
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easy() {
        let mut tokenizer = Tokenizer::new("salut");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "salut",
                word_index: 0,
                char_index: 0
            })
        );
        assert_eq!(tokenizer.next(), None);

        let mut tokenizer = Tokenizer::new("yo    ");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "yo",
                word_index: 0,
                char_index: 0
            })
        );
        assert_eq!(tokenizer.next(), None);
    }

    #[test]
    fn hard() {
        let mut tokenizer = Tokenizer::new(" .? yo lolo. aïe (ouch)");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "yo",
                word_index: 0,
                char_index: 4
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "lolo",
                word_index: 1,
                char_index: 7
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "aïe",
                word_index: 9,
                char_index: 13
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "ouch",
                word_index: 17,
                char_index: 18
            })
        );
        assert_eq!(tokenizer.next(), None);

        let mut tokenizer = Tokenizer::new("yo ! lolo ? wtf - lol . aïe ,");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "yo",
                word_index: 0,
                char_index: 0
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "lolo",
                word_index: 8,
                char_index: 5
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "wtf",
                word_index: 16,
                char_index: 12
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "lol",
                word_index: 17,
                char_index: 18
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "aïe",
                word_index: 25,
                char_index: 24
            })
        );
        assert_eq!(tokenizer.next(), None);
    }

    #[test]
    fn hard_long_chars() {
        let mut tokenizer = Tokenizer::new(" .? yo 😂. aïe");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "yo",
                word_index: 0,
                char_index: 4
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "😂",
                word_index: 1,
                char_index: 7
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "aïe",
                word_index: 9,
                char_index: 10
            })
        );
        assert_eq!(tokenizer.next(), None);

        let mut tokenizer = Tokenizer::new("yo ! lolo ? 😱 - lol . 😣 ,");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "yo",
                word_index: 0,
                char_index: 0
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "lolo",
                word_index: 8,
                char_index: 5
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "😱",
                word_index: 16,
                char_index: 12
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "lol",
                word_index: 17,
                char_index: 16
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "😣",
                word_index: 25,
                char_index: 22
            })
        );
        assert_eq!(tokenizer.next(), None);
    }

    #[test]
    fn hard_kanjis() {
        let mut tokenizer = Tokenizer::new("\u{2ec4}lolilol\u{2ec7}");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "\u{2ec4}",
                word_index: 0,
                char_index: 0
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "lolilol",
                word_index: 1,
                char_index: 1
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "\u{2ec7}",
                word_index: 2,
                char_index: 8
            })
        );
        assert_eq!(tokenizer.next(), None);

        let mut tokenizer = Tokenizer::new("\u{2ec4}\u{2ed3}\u{2ef2} lolilol - hello    \u{2ec7}");

        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "\u{2ec4}",
                word_index: 0,
                char_index: 0
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "\u{2ed3}",
                word_index: 1,
                char_index: 1
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "\u{2ef2}",
                word_index: 2,
                char_index: 2
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "lolilol",
                word_index: 3,
                char_index: 4
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "hello",
                word_index: 4,
                char_index: 14
            })
        );
        assert_eq!(
            tokenizer.next(),
            Some(Token {
                word: "\u{2ec7}",
                word_index: 5,
                char_index: 23
            })
        );
        assert_eq!(tokenizer.next(), None);
    }
}
