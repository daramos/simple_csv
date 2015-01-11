extern crate test;

use std::borrow::Cow;
use std::vec::Vec;
use std::mem::replace;
use std::io::{IoResult,IoErrorKind};
use std::default::Default;

// Reserving space for the column Strings initially seems to significantly increase performance
// Especially for column lengths <STRING_INITIAL_CAPACITY
static STRING_INITIAL_CAPACITY: usize = 64us;

enum ParseState {
    Neutral,
    InField,
    InQuotedField,
    EncounteredQuoteInQuotedField,
    EndOfRow
}


pub struct SimpleCsvReader<B: Buffer> {
    state: ParseState,
    row_data: Vec<String>,
    column_buffer: String,
    input_reader: B,
    options: SimpleCsvReaderOptions
}

#[derive(Copy)]
pub struct SimpleCsvReaderOptions {
    delimiter: char,
    text_enclosure: char
}

impl Default for SimpleCsvReaderOptions {
    fn default() -> SimpleCsvReaderOptions {
        SimpleCsvReaderOptions {
            delimiter: ',',
            text_enclosure: '"'
        }
    }
}



impl<B: Buffer> SimpleCsvReader<B> {

    pub fn new(buffer: B) -> SimpleCsvReader<B> {
        SimpleCsvReader::with_options(buffer,Default::default())
    }
    
    pub fn with_options(buffer: B, options: SimpleCsvReaderOptions)  -> SimpleCsvReader<B> {
        
        SimpleCsvReader {
            state : ParseState::Neutral,
            row_data : Vec::new(),
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
    
    fn process_line<'b>(&mut self, line : &Cow<'b, String, str>) {
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
    
    pub fn next_row<'b>(&'b mut self) -> IoResult<&'b [String]> {
        // continually read lines. The match statement below will break once the end of row is reached
        self.row_data.drain();
        self.state = ParseState::Neutral;
        let mut line_count = 0us;
        
        loop {
            let line_result = self.input_reader.read_until('\n' as u8);
            match line_result {
                Ok(ref line_bytes) => {
                    line_count += 1;
                    let line = String::from_utf8_lossy(&**line_bytes);
                    self.process_line(&line);
                    match self.state {
                        ParseState::EndOfRow => {
                            break;
                        },
                        _ => {}
                    }
                },
                Err(e) => {
                    match e.kind {
                        IoErrorKind::EndOfFile if line_count > 0 => {
                            // we've already processed a line for this row, 
                            // so instead of throwing EOF right now, return the row
                            // We'll end up returning the EOF on the next call to this function
                            
                            // The parser might have left some data in the column_buffer variable since it never encountered a newline.
                            // Add whatever was collected to the current row
                            if !self.column_buffer.is_empty() {
                                self.new_column();
                            }
                            
                            break; 
                        },
                        _ => {
                            return Err(e);
                        }
                    }
                    
                    
                }
            }
        }

        return Ok(&*self.row_data)
        
    }    
}

impl<B: Buffer> Iterator for SimpleCsvReader<B> {
    type Item = Vec<String>;
    fn next(&mut self) -> Option<Vec<String>> {
        let x = self.next_row().is_ok();
        match x {
            true => {
                let cap = self.row_data.capacity();
                let row = replace(&mut self.row_data, Vec::with_capacity(cap));
                Some(row)
            },
            false => {
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return (0,None);
    }
}

#[test]
fn reader_simple_csv_test() {
    let test_string = "1,2,3\r\n4,5,6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
    
}

#[test]
fn reader_quoted_csv_test() {
    let test_string = "1,\"2\",3\r\n4,\"5\",6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
    
}

#[test]
fn reader_quote_in_quoted_csv_test() {
    let test_string = r#"1,"""2",3"#.to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),r#""2"#.to_string(),"3".to_string()]));
    assert!(reader.next_row().is_err());
    
}

#[test]
fn reader_newline_in_quoted_csv_test() {
    let test_string = "1,\"2\",3\r\n4,\"5\r\n\",6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5\r\n".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
    
}

#[test]
fn reader_eof_in_quoted_csv_test() {
    let test_string = "1,2,3\r\n4,5,\"6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
}

#[test]
fn reader_data_after_quoted_csv_test() {
    let test_string = "1,2,3\r\n4,5,\"6\"data_after_quoted_field".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6data_after_quoted_field".to_string()]));
    assert!(reader.next_row().is_err());
}

#[test]
fn reader_newline_only_on_last_column() {
    let test_string = "1,2,3\r\n4,5,\r\n".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"".to_string()]));
    assert!(reader.next_row().is_err());
    
}

#[test]
fn reader_empty_line_in_file() {
    let test_string = "1,2,3\r\n\r\n4,5,6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
}

#[test]
fn reader_carriage_return_in_data_after_quoted_field() {
    let test_string = "1,2,\"3\"\r9\r\n4,5,6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"39".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
}

#[test]
fn reader_bad_utf8() {

    let test_string = "1,2,3\r\n4,5,6".to_string();
    let mut str_bytes = test_string.into_bytes();
    str_bytes.push(0xff);
    let test_csv_reader = &*str_bytes;
    
    let mut reader = SimpleCsvReader::new(test_csv_reader);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6\u{FFFD}".to_string()]));
}

#[test]
fn reader_different_delimiter() {

    let test_string = "1|2|3\r\n4|5|6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    let mut csv_options: SimpleCsvReaderOptions = Default::default();
    csv_options.delimiter = '|';
    let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
}

#[test]
fn reader_custom_text_enclosing_char() {
    let test_string = "1,#2#,3\r\n#4#,5,6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    let mut csv_options: SimpleCsvReaderOptions = Default::default();
    csv_options.text_enclosure = '#';
    let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
}

#[test]
fn reader_utf8_delimiter() {

    let test_string = "1\u{00A9}2\u{00A9}3\r\n4\u{00A9}5\u{00A9}6".to_string();
    let bytes = test_string.into_bytes();
    let test_csv_reader = &*bytes;
    let mut csv_options: SimpleCsvReaderOptions = Default::default();
    csv_options.delimiter = '\u{00A9}';
    let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

    assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
    assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
    assert!(reader.next_row().is_err());
}

#[bench]
fn reader_bench_throughput(b: &mut test::Bencher) {
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
        while let Ok(_) = reader.next_row() {
            x+=1;
        }
        assert_eq!(x,num_rows);
    });
}

#[bench]
fn reader_bench_throughput_long_columns(b: &mut test::Bencher) {
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
        while let Ok(_) = reader.next_row() {
            x+=1;
        }
        assert_eq!(x,num_rows);
    });
}


#[bench]
fn reader_bench_throughput_iter(b: &mut test::Bencher) {
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
        for _ in reader {
            x+=1;
        }
        assert_eq!(x,num_rows);
    });
}
