use clap::Parser;
use php_parser_rs::indexer::Indexer;
use std::{io::Result, path::PathBuf};

#[derive(Parser, Default, Debug)]
#[clap(version, about = "A PHP Parser")]
struct Arguments {
    file: String,
    #[clap(short, long)]
    /// Don't print anything
    silent: bool,
    #[clap(short, long)]
    /// Print as json
    json: bool,
    #[clap(short, long)]
    /// Generate an index from the AST and dump it out.
    index: bool,
}

fn main() -> Result<()> {
    let args = Arguments::parse();

    let file = args.file;
    let contents = std::fs::read_to_string(&file)?;
    let silent = args.silent;
    let print_json = args.json;
    let index = args.index;

    match php_parser_rs::parse(&contents) {
        Ok(ast) => {
            // if --silent is passed, don't print anything
            if silent {
                return Ok(());
            }

            // if --json is passed, print as json
            if print_json {
                match serde_json::to_string_pretty(&ast) {
                    Ok(json) => println!("{}", json),
                    Err(error) => {
                        eprintln!("Failed to convert ast to json: {}", error);

                        std::process::exit(1);
                    }
                }
            } else if index {
                let mut indexer = Indexer::new();
                match indexer.index(PathBuf::from(&file), &ast) {
                    Err(error) => {
                        println!("Indexing error: {:?}", error);
                    },
                    _ => {},
                };
                dbg!(indexer.get_index());
            } else {
                // if --json is not passed, print as text
                println!("{:#?}", ast);
            }
        }
        Err(error) => {
            println!("{}", error.report(&contents, Some(&file), true, false)?);

            std::process::exit(1);
        }
    }

    Ok(())
}
