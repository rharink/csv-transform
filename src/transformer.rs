use crate::config::{Config, ColDef};
use crate::config;
use std::io;
use csv::StringRecord;
use heck::SnakeCase;
use std::iter::{repeat};
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
    pub fn transform(&mut self) {
        let lua = Lua::new();

        lua.context(|ctx| {
            let header_info = get_header_info(&self.config, &mut self.reader);
            let config = &self.config;

            let record = self.reader.records();
            for (i, values) in record.enumerate() {
                let values = values.unwrap();
                let row = Row::new(&config, &header_info, &values, &ctx);
                if i == 0 && config.output.header {
                    self.writer.write_record(row.get_output_headers().iter()).unwrap();
                }
                if row.filter(&config.filter) {
                    self.writer.write_record(row.to_string_record().iter()).unwrap();
                }
            }
        });
    }
}

#[derive(Debug)]
struct Column<'a> {
    def: ColDef,
    value: &'a str,
}

impl<'a> Column<'a> {
    pub fn new(def: ColDef, value: &'a str) -> Column<'a> {
        Column{
            def,
            value,
        }
    }

    pub fn get_name(&self) -> &str {
        self.def.get_name()
    }

    pub fn get_value(&self) -> &str {
        self.value
    }

    pub fn get_value_with_context(&self, ctx: &Context) -> Result<String, String> {
        if let Some(f) = self.def.get_func() {
            return match ctx.load(f).eval() {
                Ok(s) => Ok(s),
                Err(e) => Err(format!("error while executing function {:?} {:?}", e, f)),
            };
        }

        return Ok(self.value.to_string());
    }

    pub fn is_excluded(&self) -> bool {
        self.def.get_exclude().unwrap_or(false)
    }
}

struct Row<'a, 'b, 'c> {
    cols: Vec<Column<'a>>,
    ctx: &'b Context<'c>,
}

impl<'a, 'b, 'c> Row<'a, 'b, 'c> {
    pub fn new(config: &Config, header_info: &HeaderInfo, values: &'a StringRecord, ctx: &'b Context<'c>) -> Row<'a, 'b, 'c> {
        let zipper = header_info.total_headers.iter().zip(values.iter().chain(repeat("")));
        let row = Row {
            cols: zipper.map(|(key, value)| {
                let the_def = match config.get_column_definition(key.as_str()) {
                    Some(def) => def.clone(),
                    None => ColDef::new(key.clone()),
                };
                Column::new(the_def, value)
            }).collect(),
            ctx,
        };
        row.set_context(ctx);
        row
    }

    fn set_context(&self, ctx: &Context) {
        let cols = &self.cols;
        let g = ctx.globals();
        cols.iter().for_each(|col|{
            g.set(col.get_name(), col.get_value()).unwrap();
        });
    }

    pub fn filter(&self, filter: &Option<String>) -> bool {
        match filter {
            Some(f) => self.ctx.load(&f).eval().unwrap_or(true),
            None => true
        }
    }

    pub fn to_string_record(&self) -> csv::StringRecord {
        self.cols.iter()
            .filter(|col| {
                !col.is_excluded()
            })
            .fold(csv::StringRecord::new(), |mut acc, col| {
                match col.get_value_with_context(&self.ctx) {
                    Ok(val) => acc.push_field(val.as_str()),
                    Err(e) => panic!(e),
                }
                acc
            })
    }

    pub fn get_output_headers(&self) -> StringRecord {
        self.cols.iter().fold(StringRecord::new(), |mut acc, col|{
            if ! col.is_excluded() {
                acc.push_field(col.get_name());
            }
            acc
        })
    }
}

#[derive(Debug)]
struct HeaderInfo {
    original_headers: Vec<String>,
    total_headers: Vec<String>
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

    let total_headers = cfg.get_headers()
        .iter()
        .fold(headers.clone(),|mut acc, header|{
            if ! acc.contains(header) {
                acc.push(header.clone().to_owned());
            }
            acc
        });

    HeaderInfo {
        original_headers: headers,
        total_headers,
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


