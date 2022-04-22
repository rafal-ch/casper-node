use std::{fs};

const SCHEMAS_DIR: &str = "src/capnp/schemas";

fn main() {
    // Compile capnp schemas
    let mut compiler = ::capnpc::CompilerCommand::new();

    let entries = fs::read_dir(SCHEMAS_DIR)
        .unwrap_or_else(|_| panic!("unable to access schema dir: {}", SCHEMAS_DIR));

    for entry in entries {
        match &entry {
            Ok(entry) => match entry.path().extension() {
                Some(extension) => {
                    if extension == "capnp" {
                        compiler.file(entry.path());
                    }
                }
                None => (),
            },
            Err(err) => panic!("error accessing 'capnp' schema: {:?} {}", entry, err),
        }
    }

    compiler.run().expect("unable to compile 'capnp' schemas")
}
