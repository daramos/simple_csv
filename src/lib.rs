#![crate_name = "simple_csv"]
#![feature(collections)]
#![feature(io)]
#![feature(test)]
#![feature(core)]

pub use reader::SimpleCsvReader;
pub use writer::SimpleCsvWriter;

pub mod reader;
pub mod writer;

#[cfg(test)]
mod tests {
    extern crate test;
    use {SimpleCsvWriter, SimpleCsvReader};

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
}
