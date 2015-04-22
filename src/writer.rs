use std::default::Default;
use std::io::{Result,Write};
use std::vec::Vec;

pub enum NewlineType {
    UnixStyle,
    WindowsStyle,
    Custom(String)
}

pub struct SimpleCsvWriterOptions {
    pub delimiter: char,
    pub text_enclosure: char,
    pub newline_type: NewlineType
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



pub struct SimpleCsvWriter<W: Write> {
    options: SimpleCsvWriterOptions,
    writer: W,
    row_written: bool
}

impl<W: Write> SimpleCsvWriter<W> {

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
    
    pub fn write(&mut self, row: &[String]) -> Result<()> {
        let delimiter = self.options.delimiter;
        let text_enclosure = self.options.text_enclosure;
        let mut col_number = 0usize;
        // Only write newline if we have already written at least one row
        if self.row_written {
            match self.options.newline_type {
                NewlineType::UnixStyle => {
                    try!(self.writer.write_all(b"\n"));
                },
                NewlineType::WindowsStyle => {
                    try!(self.writer.write_all(b"\r\n"));
                },
                NewlineType::Custom(ref newline_str) => {
                   try!(self.writer.write_all(newline_str.as_bytes()));
                }
            }
            
        }
        for column in row.iter() {
            if col_number != 0 {
                try!(write!(&mut self.writer,"{}",delimiter));
            }
            let mut is_quoted = false;
            let mut char_iterator = column.char_indices();
            let mut char_option = char_iterator.next();
            while let Some((byte_index, c)) = char_option {
                match is_quoted {
                   false => {
                            if c == text_enclosure || c == delimiter || c == '\n' || c == '\r'{
                                is_quoted = true;
                                try!(write!(&mut self.writer,"{}",text_enclosure));
                                try!(self.writer.write_all(column[..byte_index].as_bytes()));
                                // Short circuit the loop so the iterator does not get incremented
                                continue;
                            }
                    },
                    true => {
                         match c {
                            _ if c == text_enclosure  => {
                                try!(write!(&mut self.writer,"{}",c));
                                try!(write!(&mut self.writer,"{}",c));
                            },
                            _ => {
                                try!(write!(&mut self.writer,"{}",c));
                            }
                        }
                    }
                }
                // Go to the next char
                char_option = char_iterator.next();
            }
            match is_quoted {
                false => {
                    try!(self.writer.write_all(column.as_bytes()));
                },
                true => {
                    try!(write!(&mut self.writer,"{}",text_enclosure));
                }
            }
            col_number += 1;
        }
        self.row_written = true;
        Ok(())
    }
        
    
    pub fn write_all(&mut self, rows: &[Vec<String>]) -> Result<()> {
        for row in rows.iter() {
            try!(self.write(&*row));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests { 
    use super::*;
    
    #[test]
    fn writer_write_all_test() {
        let mut vec = Vec::new();
        let mut writer = SimpleCsvWriter::new(vec);
        let _ = writer.write_all(&vec![
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
        let _ = writer.write(&vec!["1".to_string(),"2\"".to_string(),"3".to_string()]);
        let _ = writer.write(&vec!["4".to_string(),"\"5".to_string(),"6".to_string()]);
        vec = writer.as_inner();
        
        let test_string = "1,\"2\"\"\",3\n4,\"\"\"5\",6";
        assert_eq!(vec, test_string.as_bytes());
        
    }

    #[test]
    fn writer_delimiter_test() {
        let mut vec = Vec::new();
        let mut writer = SimpleCsvWriter::new(vec);
        let _ = writer.write(&vec!["1".to_string(),"2,".to_string(),"3".to_string()]);
        let _ = writer.write(&vec!["4".to_string(),",5".to_string(),"6".to_string()]);
        vec = writer.as_inner();
        
        let test_string = "1,\"2,\",3\n4,\",5\",6";
        assert_eq!(vec, test_string.as_bytes());
        
    }

    #[test]
    fn writer_newline_test() {
        let mut vec = Vec::new();
        let mut writer = SimpleCsvWriter::new(vec);
        let _ = writer.write(&vec!["1".to_string(),"2\n".to_string(),"3".to_string()]);
        let _ = writer.write(&vec!["4".to_string(),",5".to_string(),"6".to_string()]);
        vec = writer.as_inner();
        
        let test_string = "1,\"2\n\",3\n4,\",5\",6";
        assert_eq!(vec, test_string.as_bytes());
        
    }
}

#[cfg(feature="nightly")]
#[cfg(test)]
mod bench {
    extern crate test;
    
    use super::*;
    use self::test::Bencher;
    
    #[bench]
    fn writer_bench_throughput(b: &mut test::Bencher) {
        let num_rows = 10000;
        let seed_vec = vec!["1".to_string(),"\"2".to_string(),"3".to_string()];
            
        let mut expected_output = Vec::new();
        let mut tmp_writer = SimpleCsvWriter::new(expected_output);
        let _ = tmp_writer.write(&seed_vec);
        expected_output = tmp_writer.as_inner();
        
        let total_bytes = expected_output.len() * num_rows;
        
        let mut test_vec = Vec::with_capacity(num_rows);
        
        for _ in (0..num_rows) {
            test_vec.push(seed_vec.clone());
        }
        
        
        b.bytes = total_bytes as u64;
        b.iter(|| {
            let r = &test_vec;
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
        let _ = tmp_writer.write(&seed_vec);
        expected_output = tmp_writer.as_inner();
        
        let total_bytes = expected_output.len() * num_rows;
        
        let mut test_vec = Vec::with_capacity(num_rows);
        
        for _ in (0..num_rows) {
            test_vec.push(seed_vec.clone());
        }
        
        
        b.bytes = total_bytes as u64;
        b.iter(|| {
            let r = &test_vec;
            let output = Vec::with_capacity(total_bytes);
            let mut writer = SimpleCsvWriter::new(output);
            let _ = writer.write_all(r);
        });
    }
}
