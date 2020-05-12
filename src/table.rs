/// Format a table with a custom number of columns, column types,
/// and rows. TableFormatter manages the width of each column and
/// formats the contents of a cell to fit within its column.
/// Cells can contain any type so long they implement the 
/// `std::fmt::Display` trait.
pub struct TableFormatter {
    width: usize, // the width, in characters, of the table
    col_widths: Vec<usize>,
    cols: Vec<String>,
}

// A single row in a table.  Pass a `TableRow` to
// `TableFormatter::print_row` which will format the cells
// into a `String`.
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

impl TableFormatter {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            col_widths: Vec::new(),
            cols: Vec::new(),
        }
    }

    /// Sets the number of columns in the table, their labels, and
    /// how wide the column is.  If not width is provided, then the
    /// width is dynamically calculated based upon the width of the
    /// table and how wide the other columns are.
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

    /// Returns a formatted string containing the label for each
    /// column positioned and formatted to align with the formatted
    /// table rows.
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

    /// Takes a single table row returns a string with each cell
    /// formatted to fit within its column.
    pub fn print_row(&self, cols: TableRow) -> String {
        use std::fmt::Write;

        // convert each row into a string
        let mut col_text = vec![];
        for i in 0..cols.row.len() {
            col_text.push(cols.row[i].to_string());
        }

        let mut longest_column = 1;
        let mut col_text_fmt = vec![];
        for i in 0..cols.row.len() {
            let text = &col_text[i];
            let fitted_text = formatting::format_to_column(&text, self.col_widths[i], 7);
            if fitted_text.len() > longest_column {
                longest_column = fitted_text.len();
            }
            col_text_fmt.push(fitted_text);
        }

        let mut row = String::new();

        for line in 0..longest_column {
            for col in 0..cols.row.len() {
                if line < col_text_fmt[col].len() {
                    write!(row, 
                        "{0: <width$}",
                        col_text_fmt[col][line].0,
                        width = self.col_widths[col]
                    ).unwrap();
                    if col_text_fmt[col][line].1 {
                        write!(row, "-").unwrap();
                    }
                } else {
                    write!(row, "{0: <width$}", "", width = self.col_widths[col]).unwrap();
                }

                if col < cols.row.len() - 1 {
                    write!(row, " ").unwrap();
                }
            }
            writeln!(row).unwrap();
        }
        row
    }
}

mod formatting {
    pub type Hyphenate = bool;
    /**
     * Takes a given string and formats it into a vector of strings
     * such that each string is no longer than the given width.  It will
     * attempt to break lines at spaces but if a word is longer than
     * the given column width it will split on the word.
     *
     * `text` - the text that needs to be formatted to fit within the width
     * of a column.
     * 
     * `width` - the width of the column in characters.
     * 
     * `split_limit` - a word must be longer than this in order to be
     * split across lines.  Unless `split_limit` > `width`, in which case,
     * it will be ignored.
     *
     * Returns a vector of tuples where the first element is a slice from
     * `text` representing a single line and the second element is a boolean
     * indicating if the line should have a hyphen appended or not.  This
     * will be `true` if a word was split across this line and the next.
     */
    pub fn format_to_column(text: &String, width: usize, split_limit: usize) -> Vec<(&str,Hyphenate)> {
        let mut breaks:Vec<(usize, usize, bool)> = vec![]; // start and length of each slice into `text`, true if midword
        let mut line_start = 0;
        let mut line_len = 0;
        let mut chars = text.chars().peekable();
        let hyphen_space = if width > 4 {1} else {0};   // If the column is wide enough to have hyphens in split words
                                                        // then this will make sure that an extra space is left to add the hyphen
        while let Some(c) = chars.next() {
            if c.is_whitespace() {
                if line_len + 1 <= width {
                    line_len += 1;
                } else {
                    breaks.push((line_start, line_len, false));
                    line_start += line_len;
                    line_len = 1;
                }
            } else {
                let word_start = line_start + line_len;
                let mut word_len = 1;
                while let Some(cp) = chars.peek() {
                    if cp.is_whitespace() {
                        break;
                    }
                    chars.next();
                    word_len += 1;
                }

                if word_len + line_len <= width {
                    line_len += word_len;
                } else {
                    let is_splittable = word_len > width || word_len > split_limit;
                    if is_splittable {
                        let adjusted_width = width - hyphen_space;
                        let split = (word_start) + if width == line_len { 0 } else {adjusted_width - line_len};
                        breaks.push((line_start, adjusted_width, split > word_start));
                        line_start = split;
                        line_len = 0;
                        word_len = word_len - (split - word_start);
                        while word_len > 0 {
                            if word_len <= adjusted_width {
                                line_len = word_len;
                                word_len = 0;
                            } else {
                                breaks.push((line_start, width, true));
                                line_start += adjusted_width;
                                word_len -= adjusted_width;
                                line_len = 0;
                            }
                        }
                    } else {
                        breaks.push((line_start, line_len, false));
                        line_start = word_start;
                        line_len = word_len;
                    }
                }
            }

            if chars.peek().is_none() && line_len > 0{
                breaks.push((line_start, line_len, false));
            }
        }

        let mut lines = vec![];
        for b in breaks {
            let start = b.0;
            let end = start + b.1;
            let line = text.get(start..end).unwrap();
            let hyphenate = hyphen_space > 0 && b.2;
            lines.push((line, hyphenate));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::formatting::*;

    #[test]
    fn split_short_words() {
        let text = String::from("the quick brown fox");
        let lines = format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!(("the quick ", false), lines[0]);
        assert_eq!(("brown fox", false), lines[1]);
    }

    #[test]
    fn split_short_words_multiple_spaces() {
        let text = String::from("the quick  brown fox   jumped   ");
        let lines = format_to_column(&text, 10, 5);
        assert_eq!(4, lines.len());
        //          1234567890    <- column numbers
        assert_eq!(("the quick ", false), lines[0]);
        assert_eq!((" brown fox", false), lines[1]);
        assert_eq!(("   jumped ", false), lines[2]);
        assert_eq!(("  ", false), lines[3]);
    }

    #[test]
    fn split_short_words_whitepsace_longer_than_column() {
        let text = String::from("the            fox");
        let lines = format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!(("the       ", false), lines[0]);
        assert_eq!(("     fox", false), lines[1]);
    }

    #[test]
    fn no_split() {
        let text = String::from("the quick");
        let lines = format_to_column(&text, 10, 5);
        assert_eq!(1, lines.len());
        //          1234567890    <- column numbers
        assert_eq!(("the quick", false), lines[0]);
    }

    #[test]
    fn split_many_words() {
        let text = String::from("the quick brown fox jumped over the lazy dog");
        let lines = format_to_column(&text, 10, 5);
        assert_eq!(5, lines.len());
        //          1234567890    <- column numbers
        assert_eq!(("the quick ", false), lines[0]);
        assert_eq!(("brown fox", false), lines[1]);
        assert_eq!(("jumped ", false), lines[2]);
        assert_eq!(("over the ", false), lines[3]);
        assert_eq!(("lazy dog", false), lines[4]);
    }

    #[test]
    fn split_word_longer_than_min_but_smaller_than_column_width() {
        let text = String::from("the quick brown fox fast jumped over the lazy dog");
        let lines = format_to_column(&text, 10, 5);
        assert_eq!(6, lines.len());
        for line in lines.iter() {
            if line.1 == false {
                assert_eq!(true, line.0.len() <= 10);
            } else {
                assert_eq!(true, line.0.len() <= 9);
            }
        }
        //          1234567890    <- column numbers
        assert_eq!(("the quick ", false), lines[0]);
        assert_eq!(("brown fox ", false), lines[1]);
        assert_eq!(("fast jump", true), lines[2]);
        assert_eq!(("ed over ", false), lines[3]);
        assert_eq!(("the lazy ", false), lines[4]);
        assert_eq!(("dog", false), lines[5]);
    }

    #[test]
    fn split_word_longer_than_column_width() {
        let text = String::from("argleybargley");
        let lines = format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!(("argleybar", true), lines[0]);
        assert_eq!(("gley", false), lines[1]);
    }

    #[test]
    fn split_word_longer_narrow_column() {
        let text = String::from("argleybargley");
        let lines = format_to_column(&text, 1, 5);
        assert_eq!(text.len(), lines.len());
        //          1234567890    <- column numbers
        for (idx, c) in text.chars().enumerate() {
            assert_eq!(format!("{}", c), lines[idx].0);
            assert_eq!(false, lines[idx].1);
        }
    }

    #[test]
    fn split_word_longer_than_column_width_shorter_than_min_word_and_too_short_add_hyphen() {
        let text = String::from("bark");
        let lines = format_to_column(&text, 3, 5);
        assert_eq!(2, lines.len());
        //          123    <- column numbers
        assert_eq!(("bar", false), lines[0]);   // the column is too narrow to add a hyphen
        assert_eq!(("k", false), lines[1]);
    }

    #[test]
    fn split_word_change_limit() {
        let text = String::from("the quick brown fox fast jumped over the lazy dog");
        let lines = format_to_column(&text, 10, 7);
        assert_eq!(6, lines.len());
        //          1234567890    <- column numbers
        assert_eq!(("the quick ", false), lines[0]);
        assert_eq!(("brown fox ", false), lines[1]);
        assert_eq!(("fast ", false), lines[2]);
        assert_eq!(("jumped ", false), lines[3]);
        assert_eq!(("over the ", false), lines[4]);
        assert_eq!(("lazy dog", false), lines[5]);
    }
}
