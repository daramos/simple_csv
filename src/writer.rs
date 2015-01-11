extern crate test;

use std::default::Default;
use std::io::IoResult;
use std::vec::Vec;

pub enum NewlineType {
    UnixStyle,
    WindowsStyle,
    Custom(String)
}

pub struct SimpleCsvWriterOptions {
    delimiter: char,
    text_enclosure: char,
    newline_type: NewlineType
}

impl Default for SimpleCsvWriterOptions {
    fn default() -> SimpleCsvWriterOptions {
        SimpleCsvWriterOptions {
            delimiter: ',',
            text_enclosure: '"',
            newline_type: NewlineType::UnixStyle
        }
    }
}



pub struct SimpleCsvWriter<W: Writer> {
    options: SimpleCsvWriterOptions,
    writer: W,
    row_written: bool
}

impl<W: Writer> SimpleCsvWriter<W> {

    pub fn new(writer: W) -> SimpleCsvWriter<W>{
        SimpleCsvWriter::with_options(writer, Default::default())
    }
    
    pub fn with_options(writer: W, options: SimpleCsvWriterOptions) -> SimpleCsvWriter<W> {
        SimpleCsvWriter {
            options: options,
            writer: writer,
            row_written: false
        }
    }
    
    pub fn as_inner(self) -> W {
        self.writer
    }
    
    pub fn write(&mut self, row: &[String]) -> IoResult<()> {
        let delimiter = self.options.delimiter;
        let text_enclosure = self.options.text_enclosure;
        let mut col_number = 0us;
        // Only write newline if we have already written at least one row
        if self.row_written {
            match self.options.newline_type {
                NewlineType::UnixStyle => {
                    try!(self.writer.write_char('\n'));
                },
                NewlineType::WindowsStyle => {
                    try!(self.writer.write_str("\r\n"));
                },
                NewlineType::Custom(ref newline_str) => {
                    try!(self.writer.write_str(&**newline_str));
                }
            }
            
        }
        for column in row.iter() {
            if col_number != 0 {
                try!(self.writer.write_char(delimiter));
            }
            let mut is_quoted = false;
            let mut char_iterator = column.char_indices();
            let mut char_option = char_iterator.next();
            while let Some((byte_index, c)) = char_option {
                match is_quoted {
                   false => {
                            if c == text_enclosure || c == delimiter || c == '\n' || c == '\r'{
                                is_quoted = true;
                                try!(self.writer.write_char(text_enclosure));
                                try!(self.writer.write_str(&column[..byte_index]));
                                // Short circuit the loop so the iterator does not get incremented
                                continue;
                            }
                    },
                    true => {
                         match c {
                            _ if c == text_enclosure  => {
                                try!(self.writer.write_char(c));
                                try!(self.writer.write_char(c));
                            },
                            _ => {
                                try!(self.writer.write_char(c));
                            }
                        }
                    }
                }
                // Go to the next char
                char_option = char_iterator.next();
            }
            match is_quoted {
                false => {
                    try!(self.writer.write_str(&**column));
                },
                true => {
                    try!(self.writer.write_char(text_enclosure));
                }
            }
            col_number += 1;
        }
        self.row_written = true;
        Ok(())
    }
        
    
    pub fn write_all(&mut self, rows: &[Vec<String>]) -> IoResult<()> {
        for row in rows.iter() {
            try!(self.write(&**row));
        }
        Ok(())
    }
}

#[test]
fn writer_write_all_test() {
    let mut vec = Vec::new();
    let mut writer = SimpleCsvWriter::new(vec);
    let _ = writer.write_all(&*vec![
        vec!["1".to_string(),"2".to_string(),"3".to_string()],
        vec!["4".to_string(),"5".to_string(),"6".to_string()]]);
    vec = writer.as_inner();
    
    let test_string = "1,2,3\n4,5,6";
    assert_eq!(vec, test_string.as_bytes());
    
}

#[test]
fn writer_quote_test() {
    let mut vec = Vec::new();
    let mut writer = SimpleCsvWriter::new(vec);
    let _ = writer.write(&*vec!["1".to_string(),"2\"".to_string(),"3".to_string()]);
    let _ = writer.write(&*vec!["4".to_string(),"\"5".to_string(),"6".to_string()]);
    vec = writer.as_inner();
    
    let test_string = "1,\"2\"\"\",3\n4,\"\"\"5\",6";
    assert_eq!(vec, test_string.as_bytes());
    
}

#[test]
fn writer_delimiter_test() {
    let mut vec = Vec::new();
    let mut writer = SimpleCsvWriter::new(vec);
    let _ = writer.write(&*vec!["1".to_string(),"2,".to_string(),"3".to_string()]);
    let _ = writer.write(&*vec!["4".to_string(),",5".to_string(),"6".to_string()]);
    vec = writer.as_inner();
    
    let test_string = "1,\"2,\",3\n4,\",5\",6";
    assert_eq!(vec, test_string.as_bytes());
    
}

#[test]
fn writer_newline_test() {
    let mut vec = Vec::new();
    let mut writer = SimpleCsvWriter::new(vec);
    let _ = writer.write(&*vec!["1".to_string(),"2\n".to_string(),"3".to_string()]);
    let _ = writer.write(&*vec!["4".to_string(),",5".to_string(),"6".to_string()]);
    vec = writer.as_inner();
    
    let test_string = "1,\"2\n\",3\n4,\",5\",6";
    assert_eq!(vec, test_string.as_bytes());
    
}

#[bench]
fn writer_bench_throughput(b: &mut test::Bencher) {
    let num_rows = 10000;
    let seed_vec = vec!["1".to_string(),"\"2".to_string(),"3".to_string()];
        
    let mut expected_output = Vec::new();
    let mut tmp_writer = SimpleCsvWriter::new(expected_output);
    let _ = tmp_writer.write(&*seed_vec);
    expected_output = tmp_writer.as_inner();
    
    let total_bytes = expected_output.len() * num_rows;
    
    let mut test_vec = Vec::with_capacity(num_rows);
    
    for _ in (0..num_rows) {
        test_vec.push(seed_vec.clone());
    }
    
    
    b.bytes = total_bytes as u64;
    b.iter(|| {
        let r = &*test_vec;
        let output = Vec::with_capacity(total_bytes);
        let mut writer = SimpleCsvWriter::new(output);
        let _ = writer.write_all(r);
    });
}
#[bench]
fn writer_bench_throughput_long_columns(b: &mut test::Bencher) {
    let num_rows = 10000;
    let seed_vec = vec!["111111111111111111111111111111111111111".to_string(),
        "\"2222222222222222222222222222222222222222222222222".to_string(),
        "33333333333333333333333333333333333333333333333".to_string()];
        
    let mut expected_output = Vec::new();
    let mut tmp_writer = SimpleCsvWriter::new(expected_output);
    let _ = tmp_writer.write(&*seed_vec);
    expected_output = tmp_writer.as_inner();
    
    let total_bytes = expected_output.len() * num_rows;
    
    let mut test_vec = Vec::with_capacity(num_rows);
    
    for _ in (0..num_rows) {
        test_vec.push(seed_vec.clone());
    }
    
    
    b.bytes = total_bytes as u64;
    b.iter(|| {
        let r = &*test_vec;
        let output = Vec::with_capacity(total_bytes);
        let mut writer = SimpleCsvWriter::new(output);
        let _ = writer.write_all(r);
    });
}
