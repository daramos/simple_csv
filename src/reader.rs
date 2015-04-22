use std::vec::Vec;
use std::mem::replace;
use std::io::{BufRead,Result};
use std::default::Default;

// Reserving space for the column Strings initially seems to significantly increase performance
// Especially for column lengths <STRING_INITIAL_CAPACITY
static STRING_INITIAL_CAPACITY: usize = 64usize;

enum ParseState {
    Neutral,
    InField,
    InQuotedField,
    EncounteredQuoteInQuotedField,
    EndOfRow
}


pub struct SimpleCsvReader<B: BufRead> {
    state: ParseState,
    row_data: Vec<String>,
    line_bytes: Vec<u8>,
    column_buffer: String,
    input_reader: B,
    options: SimpleCsvReaderOptions
}

#[derive(Copy,Clone)]
pub struct SimpleCsvReaderOptions {
    pub delimiter: char,
    pub text_enclosure: char
}

impl Default for SimpleCsvReaderOptions {
    fn default() -> SimpleCsvReaderOptions {
        SimpleCsvReaderOptions {
            delimiter: ',',
            text_enclosure: '"'
        }
    }
}


impl<B: BufRead> SimpleCsvReader<B> {

    pub fn new(buffer: B) -> SimpleCsvReader<B> {
        SimpleCsvReader::with_options(buffer,Default::default())
    }
    
    pub fn with_options(buffer: B, options: SimpleCsvReaderOptions)  -> SimpleCsvReader<B> {
        
        SimpleCsvReader {
            state : ParseState::Neutral,
            row_data : Vec::new(),
            line_bytes : Vec::new(),
            column_buffer : String::with_capacity(STRING_INITIAL_CAPACITY),
            input_reader : buffer,
            options: options
        }
    }
    
    #[inline]
    fn new_column(&mut self) {
        let column_data = replace(&mut self.column_buffer,String::with_capacity(STRING_INITIAL_CAPACITY));
        self.row_data.push(column_data);
        self.state = ParseState::Neutral;
    }
    
    fn process_line(&mut self) {
        let line = String::from_utf8_lossy(&*self.line_bytes).into_owned();
            
        let delimiter = self.options.delimiter;
        let text_enclosure = self.options.text_enclosure;
        for c in line.chars() {
            match self.state {
                ParseState::Neutral => {
                    match c {
                        _ if c==text_enclosure => { //Start of quoted field
                            self.state = ParseState::InQuotedField;
                        },
                        _ if c==delimiter => { // empty field
                            self.row_data.push(String::new());
                        },
                        '\n' => { // Newline outside of quoted field. End of row.
                            self.new_column();
                            self.state = ParseState::EndOfRow;
                        },
                        '\r' => { // Return outside of quoted field. Eat it and keep going
                        },
                        _ => { // Anything else is unquoted data
                            self.column_buffer.push(c);
                            self.state = ParseState::InField;
                        }
                    }
                },
                ParseState::InQuotedField => {
                     match c {
                        _ if c==text_enclosure => {
                            self.state = ParseState::EncounteredQuoteInQuotedField
                        },
                        _ => { //Anything else is data
                            self.column_buffer.push(c);
                        } 
                    }
                },
                ParseState::InField => {
                     match c {
                        _ if c==delimiter => {
                            self.new_column();
                        },
                        '\n' => {
                            self.new_column();
                            self.state = ParseState::EndOfRow;
                        },
                        '\r' => { // Return outside of quoted field. Eat it and keep going
                        },
                        _ => {
                            self.column_buffer.push(c);
                        }
                    }
                },
                ParseState::EncounteredQuoteInQuotedField => {
                     match c {
                        _ if c==text_enclosure => { // 2nd " in a row inside quoted field - escaped quote
                            self.column_buffer.push(c);
                            self.state = ParseState::InQuotedField;
                        },
                        _ if c==delimiter => { // Field separator, end of quoted field
                            self.new_column();
                        },
                        '\n' => { // New line, end of quoted field
                            self.new_column();
                            self.state = ParseState::EndOfRow;
                        },
                        '\r' => { // Carriage Return after quoted field. discard.
                        },
                        _ => { // data after quoted field, treat it as data and add to existing data
                            self.column_buffer.push(c);
                            self.state = ParseState::InField;
                        }
                    }
                },
                ParseState::EndOfRow => {
                    assert!(false,"Should never reach match for EndOfRow");
                },
                
            }
        }
        
    }
    
    pub fn next_row<'b>(&'b mut self) -> Option<Result<&'b [String]>> {
    
        // Reset state
        self.row_data.truncate(0);
        self.state = ParseState::Neutral;
        let mut line_count = 0usize;
        
        // continually read lines. The match statement below will break once the end of row is reached
        loop {
            // reset our line buffer
            self.line_bytes.truncate(0);
            // read (up to) new line character
            let line_result = self.input_reader.read_until('\n' as u8, &mut self.line_bytes);
            
            match line_result {
                // Read succeeded, no error & bytes read > 0
                Ok(bytes_read) if bytes_read > 0 => {
                    line_count += 1;
                    self.process_line();
                    
                    // Exit the loop if we have reached the end of the row
                    if let ParseState::EndOfRow = self.state {
                        break;
                    }
                },
                // No error, but no data read (EOF)
                Ok(..) => {
                    if line_count > 0 {
                        // we've already processed a line for this row, 
                        // so instead of returning the None right now, return the row
                        // We'll end up returning None on the next call to this function
                        
                        // The parser might have left some data in the column_buffer variable since it never encountered a newline.
                        // Add whatever was collected to the current row
                        if !self.column_buffer.is_empty() {
                            self.new_column();
                        }
                        // break to return normally
                        break; 
                    }
                    // No more data in input & no data processed on current call to next_row
                    return None;
                },
                // Read error encountered, return the error
                Err(e) => {
                    return Some(Err(e));
                }
            }
        }

        return Some(Ok(&self.row_data))
        
    }    
}

impl<B: BufRead> Iterator for SimpleCsvReader<B> {
    type Item = Result<Vec<String>>;
    
    fn next(&mut self) -> Option<Result<Vec<String>>> {
        // The function is written like this in order to avoid having to clone the row vector
        // Instead we create a new Vec with a capacity of row_data and swap the structure's vec with the new one
        return match self.next_row() {
            Some(Ok(..)) => {
                // 
                let cap = self.row_data.capacity();
                let row = replace(&mut self.row_data, Vec::with_capacity(cap));
                Some(Ok(row))
            },
            Some(Err(e)) => {
                Some(Err(e))
            },
            None => {
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return (0,None);
    }
}

#[cfg(test)]
mod tests {    
    use super::*;
    use std::default::Default;

    #[test]
    fn reader_simple_csv_test() {
        let test_string = "1,2,3\r\n4,5,6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
        
    }

    #[test]
    fn reader_quoted_csv_test() {
        let test_string = "1,\"2\",3\r\n4,\"5\",6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
        
    }

    #[test]
    fn reader_quote_in_quoted_csv_test() {
        let test_string = r#"1,"""2",3"#.to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),r#""2"#.to_string(),"3".to_string()]);
        assert!(reader.next_row().is_none());
        
    }

    #[test]
    fn reader_newline_in_quoted_csv_test() {
        let test_string = "1,\"2\",3\r\n4,\"5\r\n\",6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5\r\n".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
        
    }

    #[test]
    fn reader_eof_in_quoted_csv_test() {
        let test_string = "1,2,3\r\n4,5,\"6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
    }

    #[test]
    fn reader_data_after_quoted_csv_test() {
        let test_string = "1,2,3\r\n4,5,\"6\"data_after_quoted_field".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6data_after_quoted_field".to_string()]);
        assert!(reader.next_row().is_none());
    }

    #[test]
    fn reader_newline_only_on_last_column() {
        let test_string = "1,2,3\r\n4,5,\r\n".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"".to_string()]);
        assert!(reader.next_row().is_none());
        
    }

    #[test]
    fn reader_empty_line_in_file() {
        let test_string = "1,2,3\r\n\r\n4,5,6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
    }

    #[test]
    fn reader_carriage_return_in_data_after_quoted_field() {
        let test_string = "1,2,\"3\"\r9\r\n4,5,6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"39".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
    }

    #[test]
    fn reader_bad_utf8() {

        let test_string = "1,2,3\r\n4,5,6".to_string();
        let mut str_bytes = test_string.into_bytes();
        str_bytes.push(0xff);
        let test_csv_reader = &*str_bytes;
        
        let mut reader = SimpleCsvReader::new(test_csv_reader);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6\u{FFFD}".to_string()]);
    }

    #[test]
    fn reader_different_delimiter() {

        let test_string = "1|2|3\r\n4|5|6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        let mut csv_options: SimpleCsvReaderOptions = Default::default();
        csv_options.delimiter = '|';
        let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
    }

    #[test]
    fn reader_custom_text_enclosing_char() {
        let test_string = "1,#2#,3\r\n#4#,5,6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        let mut csv_options: SimpleCsvReaderOptions = Default::default();
        csv_options.text_enclosure = '#';
        let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
    }

    #[test]
    fn reader_utf8_delimiter() {

        let test_string = "1\u{00A9}2\u{00A9}3\r\n4\u{00A9}5\u{00A9}6".to_string();
        let bytes = test_string.into_bytes();
        let test_csv_reader = &*bytes;
        let mut csv_options: SimpleCsvReaderOptions = Default::default();
        csv_options.delimiter = '\u{00A9}';
        let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["1".to_string(),"2".to_string(),"3".to_string()]);
        assert_eq!(reader.next_row().unwrap().unwrap(), &*vec!["4".to_string(),"5".to_string(),"6".to_string()]);
        assert!(reader.next_row().is_none());
    }


}

#[cfg(feature="nightly")]
#[cfg(test)]
mod bench {
    extern crate test;
    
    use super::*;
    use self::test::Bencher;
    
    
    #[bench]
    fn reader_bench_throughput(b: &mut Bencher) {
        let num_rows = 10000;
        let seed_string = "1,\"2\",3,4,\"5\",6\r\n";
        let total_bytes = seed_string.len() * num_rows;
        
        let mut test_string = String::with_capacity(total_bytes);
        
        for _ in (0..num_rows) {
            test_string.push_str(seed_string);
        }
        
        let bytes = test_string.into_bytes();
        
        
        b.bytes = total_bytes as u64;
        b.iter(|| {
            let r = &*bytes;
            let mut x=0;
            let mut reader = SimpleCsvReader::new(r);
            while let Some(Ok(_)) = reader.next_row() {
                x+=1;
            }
            assert_eq!(x,num_rows);
        });
    }

    #[bench]
    fn reader_bench_throughput_long_columns(b: &mut Bencher) {
        let num_rows = 10000;
        let seed_string = "1222222211112,\"231231231231\",3312312312312312312,4312312312312312323123132312312313,\"53123123123123123123123213213\",6233123123123123132\r\n";
        let total_bytes = seed_string.len() * num_rows;
        
        let mut test_string = String::with_capacity(total_bytes);
        
        for _ in (0..num_rows) {
            test_string.push_str(seed_string);
        }
        
        let bytes = test_string.into_bytes();
        
        
        b.bytes = total_bytes as u64;
        b.iter(|| {
            let r = &*bytes;
            let mut x=0;
            let mut reader = SimpleCsvReader::new(r);
            while let Some(Ok(_)) = reader.next_row() {
                x+=1;
            }
            assert_eq!(x,num_rows);
        });
    }


    #[bench]
    fn reader_bench_throughput_iter(b: &mut Bencher) {
        let num_rows = 10000;
        let seed_string = "1,\"2\",3,4,\"5\",6\r\n";
        let total_bytes = seed_string.len() * num_rows;
        
        let mut test_string = String::with_capacity(total_bytes);
        
        for _ in (0..num_rows) {
            test_string.push_str(seed_string);
        }
        
        let bytes = test_string.into_bytes();
        
        
        b.bytes = total_bytes as u64;
        b.iter(|| {
            let r = &*bytes;
            let mut x=0;
            let reader = SimpleCsvReader::new(r);
            for _ in reader {
                x+=1;
            }
            assert_eq!(x,num_rows);
        });
    }
}
