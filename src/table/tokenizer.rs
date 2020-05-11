use arrayvec::ArrayString;

pub trait Tokenizer<'a, T: Eq> {
    type TokenIter: Iterator<Item = Token<T>>;

    fn tokenize(&self, intput: &'a str) -> Self::TokenIter;
}

const MAX_STACK_TERM_LEN: usize = 15;

#[derive(Debug, PartialEq)]
enum Term {
    Stack(ArrayString<[u8; MAX_STACK_TERM_LEN]>),
    Heap(String),
}

impl Term {
    #[inline]
    fn from_str(term: &str) -> Term {
        if term.len() <= MAX_STACK_TERM_LEN {
            Term::Stack(ArrayString::<[_; MAX_STACK_TERM_LEN]>::from(term).unwrap())
        } else {
            Term::Heap(term.to_string())
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Token<T: Eq> {
    term: Term,
    start_offset: usize,
    position: usize,
    ty: T,
}

impl<T: Eq> Token<T> {
    #[inline]
    pub fn from_str(term: &str, ty: T, start_offset: usize, position: usize) -> Token<T> {
        Token {
            term: Term::from_str(term),
            start_offset,
            position,
            ty,
        }
    }

    #[inline]
    pub fn term(&self) -> &str {
        match self.term {
            Term::Heap(ref s) => s.as_ref(),
            Term::Stack(ref s) => s.as_ref(),
        }
    }
}

pub struct CharTokenIter<'a, T: Eq> {
    cat: fn(char) -> T,
    input: &'a str,
    byte_offset: usize,
    char_offset: usize,
    position: usize,
}

impl<'a, T: Eq> CharTokenIter<'a, T> {
    pub fn new(cat: fn(char) -> T, input: &'a str) -> Self {
        CharTokenIter {
            cat,
            input,
            byte_offset: 0,
            char_offset: 0,
            position: 0,
        }
    }
}

impl<'a, T: Eq> Iterator for CharTokenIter<'a, T> {
    type Item = Token<T>;

    fn next(&mut self) -> Option<Token<T>> {
        let mut category = None;

        let categorizer = &self.cat;
        let catted_chars = self.input[self.byte_offset..]
            .char_indices()
            .map(|(b, c)| (b, categorizer(c)));
        for (bidx, cat) in catted_chars {
            if category.is_none() {
                category = Some(cat)
            } else if category != Some(cat) {
                // the character category has changed meaning the
                // end of the current token has been reached.
                let slice = &self.input[self.byte_offset..self.byte_offset + bidx];
                let token =
                    Token::from_str(slice, category.unwrap(), self.char_offset, self.position);
                self.position += 1;
                self.char_offset += slice.chars().count();
                self.byte_offset += bidx;

                return Some(token);
            }
        }

        if self.byte_offset < self.input.len() {
            let slice = &self.input[self.byte_offset..];
            let token = Token::from_str(slice, category.unwrap(), self.char_offset, self.position);
            self.byte_offset = self.input.len();
            Some(token)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::Term::*;

    #[derive(Debug, Eq, PartialEq)]
    enum WSToken {
        Whitespace,
        Word,
        Punctation,
    }

    pub struct WhitespaceTokenizer;

    impl<'a> Tokenizer<'a, WSToken> for WhitespaceTokenizer {
        type TokenIter = CharTokenIter<'a, WSToken>;

        fn tokenize(&self, input: &'a str) -> Self::TokenIter {
            CharTokenIter::new(is_whitespace, input)
        }
    }

    #[inline]
    fn is_whitespace(input: char) -> WSToken {
        if input.is_whitespace() {
            WSToken::Whitespace
        } else if input.is_ascii_punctuation() {
            WSToken::Punctation
        } else {
            WSToken::Word
        }
    }

    #[test]
    fn basic_tokenizer(){
        let text = "Hello, World!";
        let mut tokens = WhitespaceTokenizer.tokenize(text);
        let expected =vec![
            Token::from_str("Hello", WSToken::Word, 0, 0), 
            Token::from_str(",", WSToken::Punctation, 5, 1), 
            Token::from_str(" ", WSToken::Whitespace, 6, 2), 
            Token::from_str("World", WSToken::Word, 7, 3), 
            Token::from_str("!", WSToken::Punctation, 12, 4), 
            ];

        let ex_token = &expected[0];
        let act_token = tokens.next().unwrap();
        assert_eq!(ex_token, &act_token);

        let ex_token = &expected[1];
        let act_token = tokens.next().unwrap();
        assert_eq!(ex_token, &act_token);

        let ex_token = &expected[2];
        let act_token = tokens.next().unwrap();
        assert_eq!(ex_token, &act_token);

        let ex_token = &expected[3];
        let act_token = tokens.next().unwrap();
        assert_eq!(ex_token, &act_token);

        let ex_token = &expected[4];
        let act_token = tokens.next().unwrap();
        assert_eq!(ex_token, &act_token);

        let act_token = tokens.next();
        assert_eq!(None, act_token);
    }

    #[test]
    fn empty_string() {
        let text = "";
        let mut tokens = WhitespaceTokenizer.tokenize(text);
        assert_eq!(None, tokens.next());
    }

    #[test]
    fn one_token() {
        let text = "Hello";
        let mut tokens = WhitespaceTokenizer.tokenize(text);
        let expected = Token::from_str("Hello", WSToken::Word, 0, 0);

        let act_token = tokens.next().unwrap();
        assert_eq!(&expected, &act_token);
    }
}