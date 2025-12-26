# WADUP

## About

Web Assembly Data Unified Processing (WADUP) is a framework for extracting sub-content and metadata from content.

## Components

### Processor

The processor is the core application which takes content as input and outputs sub-content and metadata

Key features:

- Written in Rust
- Uses the wasmtime library to run user-provided WASM module which run in a sandbox
- When launching the processor, a CLI argument is provided which points to a folder full of user-provided WASM modules
- At startup, all user-provided WASM modules are loaded and stay loaded until the application finishes
- The only bindings/functions exposed to the WASM module are those necessary to facilitate taking content as input and outputting sub-content and metadata
- Initial input content is provided by pointing the application at a folder. Each file in that folder is processed as a new piece of content
- When sub-content is output, it is recursively fed back in to be processed as new content.
- The application finishes when all content and recursively generated sub-content has been processed.
- Metadata consists of tables with rows and columns.
- The WASM bindings have functions to define a table and it's columns. Supported data-types are int64, float64 and string.
- The WASM bindings also have functions to insert new rows into the tables.
- Metadata will be written to a SQLite database and sub-content will not be written anywhere.
- All content (and sub-content) will be assigned a UUID for data provinance. All metadata tables will contain that UUID so that the metadata can be traced back to the content.
- There will be a metadata table (in addition to the user-created tables) that contains the filename of the content and the parent content UUID (if applicable)
- For sub-content, the user-provided WASM module can provide the filename
- When the user-provided WASM module is outputing sub-content, it can either write bytes or provide an offset and length in the content it is parsing.
- Where possible, the processor should avoid copying data and it should operate in-memory only.
- Content and sub-content is processed depth first so that all recursive sub-content is processed before moving on to load more new content from files.
- The content should be loaded once, and then processed by all of the loaded WASM modules

### WASM Module Libraries

WASM Module Libraries are helper libraries to make creating WASM modules compatible with the WADUP system ergonomic and idiomatic in different languages.

To start with, there will only be a Rust library.

## Tests

There will be integration tests for the following:

- A single input file which is a SQLite database (populate it with sample data). A single WASM module written in Rust, using the Rust WASM module library. The WASM module should first identify if the file is a SQLite database. If it is, parse the SQLite database and output metadata of the number of rows per table.
- A single input file which is a ZIP file containing 2 files. Each file contains some random text. There should be two WASM modules written in Rust, using the Rust WASM module library. One module which identifies whether the content is a ZIP file and, if it is, unzips it, outputing each file inside as separate sub-content. The other module outputs the size of the content (number of bytes) as metadata.
- Both of the above tests combined. So two input files and three WASM modules, all run at once.