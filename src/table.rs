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

        let mut longest_column = 1;
        let mut col_text = vec![];
        for i in 0..cols.row.len() {
            let text = cols.row[i].to_string().clone();
            let fitted_text = TableFormatter::format_to_column(&text, self.col_widths[i], 7);
            if fitted_text.len() > longest_column {
                longest_column = fitted_text.len();
            }
            col_text.push(fitted_text);
        }

        let mut row = String::new();

        for line in 0..longest_column {
            for col in 0..cols.row.len() {
                if line < col_text[col].len() {
                    write!(row, 
                        "{0: <width$}",
                        col_text[col][line],
                        width = self.col_widths[col]
                    ).unwrap();
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

    /**
     * Takes a given string and formats it into a vector of strings
     * such that each string is no longer than the given width.  It will
     * attempt to break lines at spaces but if a word is longer than
     * the given column width it will split on the word.
     */
    fn format_to_column(text: &String, width: usize, split_limit: usize) -> Vec<String> {
        let mut breaks:Vec<(usize, usize)> = vec![]; // start and length of each slice into `text`
        let mut line_start = 0;
        let mut line_len = 0;

        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c.is_whitespace() {
                if line_len + 1 <= width {
                    line_len += 1;
                } else {
                    breaks.push((line_start, line_len));
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
                        breaks.push((line_start, width));
                        let split = word_start + (width - line_len);
                        line_start = split;
                        line_len = 0;
                        word_len = word_len - (split - word_start);
                        while word_len > 0 {
                            if word_len <= width {
                                line_len = word_len;
                                word_len = 0;
                            } else {
                                breaks.push((line_start, width));
                                line_start += width;
                                word_len -= width;
                                line_len = 0;
                            }
                        }
                    } else {
                        breaks.push((line_start, line_len));
                        line_start = word_start;
                        line_len = word_len;
                    }
                }
            }

            if chars.peek().is_none() && line_len > 0{
                breaks.push((line_start, line_len));
            }
        }

        let mut lines = vec![];
        for b in breaks {
            let start = b.0;
            let end = start + b.1;
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
        assert_eq!(" brown fox", lines[1]);
        assert_eq!("   jumped ", lines[2]);
        assert_eq!("  ", lines[3]);
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
        assert_eq!(5, lines.len());
        for line in lines.iter() {
            assert_eq!(true, line.len() <= 10);
        }
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox ", lines[1]);
        assert_eq!("fast jumpe", lines[2]);
        assert_eq!("d over the", lines[3]);
        assert_eq!(" lazy dog", lines[4]);
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
    fn split_word_longer_narrow_column() {
        let text = String::from("argleybargley");
        let lines = TableFormatter::format_to_column(&text, 1, 5);
        assert_eq!(text.len(), lines.len());
        //          1234567890    <- column numbers
        for (idx, c) in text.chars().enumerate() {
            assert_eq!(format!("{}", c), lines[idx]);
        }
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
