use crate::config::{Config, ColDef};
use crate::config;
use std::io;
use csv::StringRecord;
use std::error::Error;
use heck::SnakeCase;
use std::convert::identity;
use std::collections::HashSet;
use std::str::FromStr;
use std::iter::{Zip, repeat};
use rlua::{Context, Lua};

pub struct Transformer {
    reader: csv::Reader<Box<dyn io::Read>>,
    writer: csv::Writer<Box<dyn io::Write>>,
    config: Config,
}

impl Transformer {
    /// Creates a new transformer
    pub fn new(cfg: Config, rdr: Box<dyn io::Read>, wtr: Box<dyn io::Write>) -> Transformer {
        Transformer {
            reader: build_reader(&cfg.input, rdr),
            writer: build_writer(&cfg.output, wtr),
            config: cfg,
        }
    }

    /// Transform transforms the csv as configured
    pub fn transform(&mut self) -> Result<(), String> {
        let header_info = get_header_info(&self.config, &mut self.reader);
        let lua = Lua::new();

        if self.config.output.header {
            self.writer.write_record(header_info.output_headers.iter());
        }

        for values in self.reader.records() {
            //let row = get_row(&self.config, raw_row.unwrap(), &header_info);
            let config = &self.config;
            let out =
                header_info.output_headers.iter().zip(
                    // Chain the iter of values with a repeat so extra columns get a default value
                    // and appear in the map and fold below.
                    values.unwrap().iter().chain(repeat(""))

                // Set all row values as globals in the Lua context
                ).map(|(key, value)| {
                    lua.context(|ctx| {
                        let g = ctx.globals();
                        g.set(key.as_str(), value);
                    });
                    (key, value)

                // Filter the row based on a lua function
                }).filter(|_|{
                    lua.context(|ctx| {
                        if let Some(f) = &config.filter {
                            ctx.load(f).eval().unwrap_or(true)
                        } else {
                            true
                        }
                })

                // Create a new string record from possibly transformed values.
                }).fold(StringRecord::new(), |mut acc, (key, value)| {
                    if let Some(def) = &config.get_column_definition(key.as_str()) {
                        lua.context(|ctx| {
                            acc.push_field(transform_value(&ctx, def.get_func(), value).as_str());
                        });
                    } else {
                        acc.push_field(value);
                    }
                    acc
                });

            // Write the record to the output
            self.writer.write_record(out.iter());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct HeaderInfo {
    original_headers: Vec<String>,
    output_headers: Vec<String>
}

fn get_header_info(cfg: &Config, rdr: &mut csv::Reader<Box<dyn io::Read>>) -> HeaderInfo {
    let headers = rdr.headers()
        .expect("cannot read header information")
        .iter()
        .map(|x| {
            x.to_snake_case()
        })
        .fold(vec![], |mut acc, header| {
            acc.push(header);
            acc
        });

    let output_headers = cfg.get_headers()
        .iter()
        .fold(headers.clone(),|mut acc, header|{
            if ! acc.contains(header) {
                acc.push(header.clone().to_owned());
            }
            acc
        });

    HeaderInfo {
        original_headers: headers,
        output_headers,
    }
}

/// Builds a csv reader from a io::Read and configuration.
fn build_reader(cfg: &config::Input, rdr: Box<dyn io::Read>) -> csv::Reader<Box<dyn io::Read>> {
    let mut builder = csv::ReaderBuilder::new();

    builder.quoting(cfg.quoting);

    builder.double_quote(cfg.double_quote);

    if cfg.trim {
        builder.trim(csv::Trim::All);
    }

    if let Some(q) = cfg.quote {
        builder.quote(q as u8);
    }

    if let Some(e) = cfg.escape {
        builder.escape(Some(e as u8));
    }

    if let Some(c) = cfg.comment {
        builder.comment(Some(c as u8));
    }

    if let Some(d) = cfg.delimiter {
        builder.delimiter(d as u8);
    }

    if let Some(t) = cfg.terminator {
        builder.terminator(csv::Terminator::Any(t as u8));
    }

    builder.from_reader(rdr)
}

/// Builds a csv writer from a io::Write and configuration.
fn build_writer(cfg: &config::Output, wtr: Box<dyn io::Write>) -> csv::Writer<Box<dyn io::Write>> {
    let mut builder = csv::WriterBuilder::new();

    builder.double_quote(cfg.double_quote);

    if let Some(q) = cfg.quote {
        builder.quote(q as u8);
    }

    if let Some(e) = cfg.escape {
        builder.escape(e as u8);
    }

    if let Some(d) = cfg.delimiter {
        builder.delimiter(d as u8);
    }

    if let Some(t) = cfg.terminator {
        builder.terminator(csv::Terminator::Any(t as u8));
    }

    builder.from_writer(wtr)
}

fn transform_value(ctx: &Context, func: &Option<String>, value: &str) -> String {
    if let Some(f) = func {
        match ctx.load(f).eval::<String>() {
            Ok(v) => return v,
            Err(e) => panic!("error while executing function {:?} {:?}", e, func),
        }
    }

    value.to_string()
}

