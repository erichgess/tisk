pub struct TableRow<'a> {
    row: Vec<Box<dyn std::fmt::Display + 'a>>,
}

impl<'a> TableRow<'a> {
    pub fn new() -> Self {
        Self { row: Vec::new() }
    }

    pub fn push<S: std::fmt::Display + 'a>(&mut self, col: S) {
        self.row.push(Box::new(col))
    }
}

pub struct TableFormatter {
    width: usize, // the width, in characters, of the table
    col_widths: Vec<usize>,
    cols: Vec<String>,
}

impl TableFormatter {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            col_widths: Vec::new(),
            cols: Vec::new(),
        }
    }

    pub fn set_columns(&mut self, cols: Vec<(&str, Option<usize>)>) {
        // Add up the widths of the explicitly defined columns
        // adding 1 to account for a space between each column
        let with_width = cols.iter().filter(|x| x.1.is_some());
        let allocated_width: usize = with_width.map(|x| x.1.unwrap() + 1).sum();

        // Count the number of columsn without a width
        let without_width = cols.iter().filter(|x| x.1.is_none());
        let num_without_width = without_width.count();

        // Get the amount of space which is not explicitly assigned to a column
        // Divide evenly between the columns without width
        if self.width < allocated_width {
            panic!("Total width of columns is greater than the width of the table")
        }
        let remaining_space = self.width - allocated_width;
        let width_per_col = remaining_space / num_without_width;

        // Record the columns and their widths
        for (label, width) in cols {
            match width {
                Some(w) => {
                    self.cols.push(String::from(label));
                    self.col_widths.push(w);
                }
                None => {
                    self.cols.push(String::from(label));
                    self.col_widths.push(width_per_col);
                }
            }
        }
    }

    pub fn print_header(&self) {
        use console::Style;
        let ul = Style::new().underlined();
        let num_cols = self.cols.len();
        for i in 0..num_cols {
            print!(
                "{0: <width$}",
                ul.apply_to(&self.cols[i]),
                width = self.col_widths[i]
            );
            if i < num_cols - 1 {
                print!(" ");
            }
        }
        println!();
    }

    pub fn print_row(&self, cols: TableRow) {
        let mut longest_column = 1;
        let mut col_text = vec![];
        for i in 0..cols.row.len() {
            let text = cols.row[i].to_string().clone();
            let fitted_text = TableFormatter::format_to_column(&text, self.col_widths[i], 7);
            if fitted_text.len() > longest_column {
                longest_column = fitted_text.len();
            }
            col_text.push(fitted_text.clone());
        }

        for line in 0..longest_column {
            for col in 0..cols.row.len() {
                if line < col_text[col].len() {
                    print!(
                        "{0: <width$}",
                        col_text[col][line],
                        width = self.col_widths[col]
                    );
                } else {
                    print!("{0: <width$}", "", width = self.col_widths[col]);
                }

                if col < cols.row.len() - 1 {
                    print!(" ");
                }
            }
            println!();
        }
    }

    /**
     * Takes a given string and formats it into a vector of strings
     * such that each string is no longer than the given width.  It will
     * attempt to break lines at spaces but if a word is longer than
     * the given column width it will split on the word.
     */
    fn format_to_column(text: &String, width: usize, split_limit: usize) -> Vec<String> {
        let mut index = 0;
        let mut chars = text.chars();
        let mut breaks = vec![];
        let mut start = 0;
        let mut end = 0;
        let mut word_start = 0;
        let mut word_end;

        while let Some(c) = chars.next() {
            index += 1;

            // if is whitespace then we are at the end of a word
            //    if word + length of current line < width then add word to line
            //    if else if word > width then hyphenate word
            //    else start new line and add word to that
            if c.is_whitespace() || index == text.len() || (index - word_start) > width {
                word_end = index; // whitespace will be added to the current word until a new word starts or the end of the column is reached
                let word_len = word_end - word_start;

                if word_len + (end - start) <= width {
                    end = word_end;
                    if index == text.len() {
                        breaks.push((start, end));
                    }
                } else {
                    let splittable = if split_limit < width {
                        word_len > split_limit
                    } else {
                        true
                    };
                    if splittable && word_len + (end - start) > width {
                        end = word_start + (width - (end - start));
                        breaks.push((start, end));
                        start = end;
                        end = word_end;
                    } else {
                        breaks.push((start, end));
                        start = word_start;
                        end = word_end;
                    }
                    if end == text.len() {
                        breaks.push((start, end));
                    }
                }

                word_start = word_end;
            }
        }

        let mut lines = vec![];
        for b in breaks {
            let start = b.0;
            let end = if b.1 > text.len() { text.len() } else { b.1 };
            lines.push(text.get(start..end).unwrap().into());
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_short_words() {
        let text = String::from("the quick brown fox");
        let lines = TableFormatter::format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox", lines[1]);
    }

    #[test]
    fn split_short_words_multiple_spaces() {
        let text = String::from("the quick  brown fox   jumped   ");
        let lines = TableFormatter::format_to_column(&text, 10, 5);
        assert_eq!(4, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!(" brown ", lines[1]);
        assert_eq!("fox   jump", lines[2]);
        assert_eq!("ed   ", lines[3]);
    }

    #[test]
    fn split_short_words_whitepsace_longer_than_column() {
        let text = String::from("the            fox");
        let lines = TableFormatter::format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the       ", lines[0]);
        assert_eq!("     fox", lines[1]);
    }

    #[test]
    fn no_split() {
        let text = String::from("the quick");
        let lines = TableFormatter::format_to_column(&text, 10, 5);
        assert_eq!(1, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick", lines[0]);
    }

    #[test]
    fn split_many_words() {
        let text = String::from("the quick brown fox jumped over the lazy dog");
        let lines = TableFormatter::format_to_column(&text, 10, 5);
        assert_eq!(5, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox ", lines[1]);
        assert_eq!("jumped ", lines[2]);
        assert_eq!("over the ", lines[3]);
        assert_eq!("lazy dog", lines[4]);
    }

    #[test]
    fn split_word_longer_than_min_but_smaller_than_column_width() {
        let text = String::from("the quick brown fox fast jumped over the lazy dog");
        let lines = TableFormatter::format_to_column(&text, 10, 5);
        assert_eq!(6, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox ", lines[1]);
        assert_eq!("fast jumpe", lines[2]);
        assert_eq!("d over ", lines[3]);
        assert_eq!("the lazy ", lines[4]);
        assert_eq!("dog", lines[5]);
    }

    #[test]
    fn split_word_longer_than_column_width() {
        let text = String::from("argleybargley");
        let lines = TableFormatter::format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("argleybarg", lines[0]);
        assert_eq!("ley", lines[1]);
    }

    #[test]
    fn split_word_longer_than_column_width_shorter_than_min_word() {
        let text = String::from("bark");
        let lines = TableFormatter::format_to_column(&text, 3, 5);
        assert_eq!(2, lines.len());
        //          123    <- column numbers
        assert_eq!("bar", lines[0]);
        assert_eq!("k", lines[1]);
    }

    #[test]
    fn split_word_change_limit() {
        let text = String::from("the quick brown fox fast jumped over the lazy dog");
        let lines = TableFormatter::format_to_column(&text, 10, 7);
        assert_eq!(6, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox ", lines[1]);
        assert_eq!("fast ", lines[2]);
        assert_eq!("jumped ", lines[3]);
        assert_eq!("over the ", lines[4]);
        assert_eq!("lazy dog", lines[5]);
    }
}
