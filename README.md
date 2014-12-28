# Simple CSV Parser

This is a CSV (delimiter can be changed) parser with a focus on:
  1. Simplicity
  2. Robustness
  3. Performance (to a lesser extent)

It follows RFC 4180, but allows for non-conformant files to be processed. 
In order to accomplish this, it makes the following assumptions:

  1. Commas on the end of a line results in a empty string for that column.
    * `1,2,3,` is parsed as `["1","2","3",""]`
  2. Double quotes in a field that is not enclosed in double quotes are processed as a regular character and are included in the column string.
    * `1,2",3` is parsed as `["1","2\"","3"]`
  3. Non-delimiter characters immediately following a quoted field are treated as part of the column data and are appended to the column string.
    * `1,2,"3"123` is parsed as `["1","2","3123"]`
  4. An EOF in the middle of a quoted field is parsed as if the field was properly closed.
    * `1,2,"3*EOF*` is parsed as `["1","2","3"]`
  5. There is no error for empty lines or varying number of columns per line.
    * An empty line is parsed as `[]`

## Limitations
  * It's currently not robust on bad utf8 and throws an error. This will be fixed soon.
  * The iterator implementation forces an allocation for every row.

## Usage
Add to your Cargo.toml:

```
[dependencies]
simple_csv = "~0.0.1"
```

### Simple CSV usage
```rust
let test_string = "1,2,3\r\n4,5,6".to_string();
let bytes = test_string.into_bytes();
let mut test_csv_reader = bytes.as_slice();

let mut parser = SimpleCsv::new(&mut test_csv_reader);

assert_eq!(parser.next_row(), Ok(vec!["1".to_string(),"2".to_string(),"3".to_string()].as_slice()));
assert_eq!(parser.next_row(), Ok(vec!["4".to_string(),"5".to_string(),"6".to_string()].as_slice()));
assert!(parser.next_row().is_err());
```
### Different Delimiter
```rust
let test_string = "1|2|3\r\n4|5|6".to_string();
let bytes = test_string.into_bytes();
let mut test_csv_reader = bytes.as_slice();

let mut parser = SimpleCsv::with_delimiter(&mut test_csv_reader,'|');

assert_eq!(parser.next_row(), Ok(vec!["1".to_string(),"2".to_string(),"3".to_string()].as_slice()));
assert_eq!(parser.next_row(), Ok(vec!["4".to_string(),"5".to_string(),"6".to_string()].as_slice()));
assert!(parser.next_row().is_err());
```

### Using a iterator
```rust
let test_string = "1|2|3\r\n4|5|6".to_string();
let bytes = test_string.into_bytes();
let mut test_csv_reader = bytes.as_slice();

let mut parser = SimpleCsv::with_delimiter(&mut test_csv_reader,'|');

for row in parser {
	println!("{}",row);
}
```

## To Do

  * Improve malformed UTF8 handling
  * Implement CSV writer?
  * Allow the iterator method to return errors somehow?
  

