# Simple CSV Library
[![Build status](https://api.travis-ci.org/daramos/simple_csv.png)](https://travis-ci.org/daramos/simple_csv)

This is a CSV (delimiter can be changed) parser & writer with a focus on:
  1. Simplicity
  2. Robustness
  3. Performance (to a lesser extent)

## Parser
The parser follows RFC 4180, but allows for non-conformant files to be processed.

In order to achieve this robustness, the parser makes the following assumptions:

  1. Commas on the end of a line results in a empty string for that column.
    * `1,2,3,` is parsed as `["1","2","3",""]`
  2. Double quotes in a field that is not enclosed in double quotes are processed as a regular character and are included in the column string.
    * `1,2",3` is parsed as `["1","2\"","3"]`
  3. Non-delimiter characters immediately following a quoted field are treated as part of the column data and are appended to the column string.
    * `1,2,"3"123` is parsed as `["1","2","3123"]`
  4. An EOF in the middle of a quoted field is parsed as if the field was properly closed.
    * `1,2,"3*EOF*` is parsed as `["1","2","3"]`
  5. There is no error for empty lines or varying number of columns per line.
    * An empty line is parsed as `[""]`
  6. Lines are assumed to be UTF8 and are decoded "lossily" via Rust's `String::from_utf8_lossy` function.
  7. The return character `\r` in unquoted fields is always discarded.


## Writer
The writer always produces RFC 4180 compliant output and can write to any object that implements the `std::io::Writer` trait.

## Usage
Add to your Cargo.toml:

```
[dependencies]
simple_csv = "~0.0.8"
```

## Simple CSV Parsing usage
```rust
let test_string = "1,2,3\r\n4,5,6".to_string();
let bytes = test_string.into_bytes();
let test_csv_reader = &*bytes;

let mut reader = SimpleCsvReader::new(test_csv_reader);

assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
assert!(reader.next_row().is_err());
```
#### Different Delimiter
```rust
let test_string = "1|2|3\r\n4|5|6".to_string();
let bytes = test_string.into_bytes();
let test_csv_reader = &*bytes;
let mut csv_options: SimpleCsvReaderOptions = Default::default();
csv_options.delimiter = '|';
let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
assert!(reader.next_row().is_err());
```

#### Using a iterator
```rust
let test_string = "1,2,3\r\n4,5,6".to_string();
let bytes = test_string.into_bytes();
let test_csv_reader = &*bytes;

let mut reader = SimpleCsvReader::new(test_csv_reader);

for row in reader {
	println!("{}",row);
}
```

#### Different Text Enclosing Character
```rust
let test_string = "1,#2#,3\r\n#4#,5,6".to_string();
let bytes = test_string.into_bytes();
let test_csv_reader = &*bytes;
let mut csv_options: SimpleCsvReaderOptions = Default::default();
csv_options.text_enclosure = '#';
let mut reader = SimpleCsvReader::with_options(test_csv_reader,csv_options);

assert_eq!(reader.next_row(), Ok(&*vec!["1".to_string(),"2".to_string(),"3".to_string()]));
assert_eq!(reader.next_row(), Ok(&*vec!["4".to_string(),"5".to_string(),"6".to_string()]));
assert!(reader.next_row().is_err());
```

## Simple CSV Writing Usage
```rust
let mut vec = Vec::new();
let mut writer = SimpleCsvWriter::new(vec);
let _ = writer.write_all(&vec![
    vec!["1".to_string(),"2".to_string(),"3".to_string()],
    vec!["4".to_string(),"5".to_string(),"6".to_string()]]);
vec = writer.as_inner();

let test_string = "1,2,3\n4,5,6";
assert_eq!(vec, test_string.as_bytes());
```

## To Do
  * Allow the iterator method to return errors
  

