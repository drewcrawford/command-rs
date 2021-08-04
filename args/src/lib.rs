mod parse;

use proc_macro::TokenStream;
/**
This is a small macro that parses args like
```rust
# use args::args;
let my_arr = args!(foo bar baz "multi word parse");
assert_eq!(my_arr, ["foo","bar","baz","multi word parse"]);
```

The following characters are reserved: `~`#$&*()\|[]{};'"<>/?!`
*/
#[proc_macro]
pub fn args(stream: TokenStream) -> TokenStream {
    let string = stream.to_string();
    let result = parse::parse_unquoted(&string);
    if result.is_err() {
        format!("compile_error!(\"{}\")",result.err().unwrap().to_string()).parse().unwrap()
    }
    else {
        format!("{:?}",result.unwrap()).parse().unwrap()
    }
}