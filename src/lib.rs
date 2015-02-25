#![crate_name = "simple_csv"]
#![feature(collections, old_io, test)]

pub use reader::SimpleCsvReader;
pub use reader::SimpleCsvReaderOptions;

pub use writer::SimpleCsvWriter;
pub use writer::SimpleCsvWriterOptions;
pub use writer::NewlineType;


pub mod reader;
pub mod writer;

#[cfg(test)]
mod tests {
    extern crate test;
    use std::default::Default;
    use super::*;

    #[test]
    fn reader_and_writer_test() {
        let data = vec![
            vec!["1".to_string(),"2\r\n".to_string(),"3".to_string()],
            vec!["4".to_string(),"5\"".to_string(),"6".to_string()]];
        let mut writer = SimpleCsvWriter::new(Vec::new());
        let _ = writer.write_all(&*data);

        let written_data = writer.as_inner();

        let mut reader = SimpleCsvReader::new(&*written_data);
        assert_eq!(reader.next_row(), Ok(&*data[0]));
        assert_eq!(reader.next_row(), Ok(&*data[1]));
        assert!(reader.next_row().is_err());

    }

    #[test]
    fn reader_and_writer_custom_delimiter_test() {
        let data = vec![
            vec!["1".to_string(),"2\r\n".to_string(),"3".to_string()],
            vec!["4".to_string(),"5\"".to_string(),"6".to_string()]];

        let mut writer_options: SimpleCsvWriterOptions = Default::default();
        writer_options.delimiter = '#';

        let mut writer = SimpleCsvWriter::with_options(Vec::new(),writer_options);
        let _ = writer.write_all(&*data);

        let written_data = writer.as_inner();

        let mut reader_options: SimpleCsvReaderOptions = Default::default();
        reader_options.delimiter = '#';

        let mut reader = SimpleCsvReader::with_options(&*written_data, reader_options);
        assert_eq!(reader.next_row(), Ok(&*data[0]));
        assert_eq!(reader.next_row(), Ok(&*data[1]));
        assert!(reader.next_row().is_err());

    }
}
