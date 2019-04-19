extern crate csv;
extern crate heck;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
extern crate clap;

use std::io;
use std::fs;

use clap::{Arg, App};
use rlua::{Lua, Context};
use heck::SnakeCase;
use csv::{StringRecord, StringRecordIter};
use std::iter::Zip;
use std::collections::HashMap;
use std::convert::identity;

#[derive(Deserialize, Debug, Clone)]
struct Config {
    version: String,
    print_header: bool,
    columns: Vec<ColumnConfig>
}

#[derive(Deserialize, Debug, Clone)]
struct ColumnConfig {
    name: String,
    exclude: bool,
    func: Option<String>
}

type Row = Vec<Column>;

#[derive(Debug)]
struct Column {
    config: ColumnConfig,
    value: Option<String>,
}

impl Column {
    fn new(config: ColumnConfig, value: Option<String>) -> Column {
        Column {
            config,
            value,
        }
    }

    fn transform_value(&self, ctx: &Context) -> Result<String, String> {
        let mut normalized_value = self.value.clone().unwrap_or(String::new());
        if let Some(func) = self.config.get_func() {
            match ctx.load(&func).eval::<String>() {
                Ok(v) => normalized_value = v,
                Err(e) => panic!("error while executing function {:?} {:?}", e, func),
            }
        }

        return Ok(normalized_value);
    }
}

impl Config {
    fn new() -> Config {
        Config {
            version: "1".to_string(),
            print_header: true,
            columns: vec![],
        }
    }

    fn get_column(&self, name: &str) -> Option<&ColumnConfig> {
        self.columns.iter().find(|c| {
            c.name == name
        })
    }

    fn get_column_names(&self) -> Vec<&str> {
        self.columns.iter().fold(vec![], |mut acc, col| {
            acc.push(col.name.as_str());
            return acc;
        })
    }
}

impl ColumnConfig {
    fn new(name: &str) -> ColumnConfig {
        ColumnConfig {
            name: name.to_string(),
            exclude: false,
            func: None,
        }
    }

    fn get_func(&self) -> &Option<String> {
        &self.func
    }
}

fn get_row_map(z: Zip<StringRecordIter, StringRecordIter>) -> HashMap<String, String> {
    z.fold(HashMap::new(), |mut acc, (key, value)| {
        acc.insert(key.to_string(), value.to_string());
        return acc;
    })
}

fn main() {
    let matches = App::new("csv-transform")
        .version("0.1.0")
        .author("Robert den Harink <robert@robhar.com")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .value_name("FILE")
            .help("file to load")
            .takes_value(true))
        .get_matches();

    let config = match matches.value_of("config") {
        Some(file_name) => read_config(file_name),
        None => Config::new(),
    };

//    let mut rdr: csv::Reader<dyn io::Read + 'static> = match matches.value_of("file") {
//        Some(file_path) => csv::Reader::from_path(file_path).expect("cannot read file"),
//        None => csv::Reader::from_reader(io::stdin()),
//    };

    let mut rdr = csv::Reader::from_reader(io::stdin());
    let mut wtr = csv::Writer::from_writer(io::stdout());
    let original_headers = read_header_information(&mut  rdr);
    transform(original_headers, &config, &mut rdr, &mut wtr);
}

fn transform(headers: StringRecord, config: &Config, rdr: &mut csv::Reader<io::Stdin>, wtr: &mut csv::Writer<io::Stdout>) {
    let lua = Lua::new();

//    if config.print_header {
//        wtr.write_record(get_headers(&row).iter()).expect("cannot write header");
//    }

    for data in rdr.records() {
        let row = create_row(&headers, data.unwrap(), config);
        set_context(&row, &lua);
        let new_data = row.iter().fold(StringRecord::new(),|mut acc, col| {
            let mut new_value = col.value.clone();
            lua.context(|ctx| {
                new_value = col.transform_value(&ctx).ok()
            });

            if !col.config.exclude {
                acc.push_field(new_value.unwrap_or(String::new()).as_str());
            }

            acc
        });


        wtr.write_record(new_data.iter()).expect("cannot write record");
     }
}

fn create_row(headers: &StringRecord, data: StringRecord, config: &Config) -> Row {
    let mapping = get_row_map(headers.iter().zip(data.iter()));

    // Create columns for existing headers/columns
    let mut original = headers.iter().fold(vec![], |mut acc, header| {
        let val = get_value(&mapping, header);
        let col = match config.get_column(header) {
            Some(cfg) => Column::new(cfg.clone().to_owned(), val),
            None => Column::new(ColumnConfig::new(header), val),
        };
        acc.push(col);
        acc
    });

    // Get the difference between the original headers and headers defined in the config
    // create columns for these accordingly.
    let mut extra = array_diff(
        config.get_column_names(),
        headers.iter().map(identity).collect())
        .iter()
        .fold(vec![], |mut acc, header| {
            acc.push(Column::new(config.get_column(header).unwrap().clone().to_owned(), None));
            acc
        });

    // Return the combined list
    original.append(&mut extra);
    original
}

fn get_value(m: &HashMap<String,String>, k: &str) -> Option<String> {
    match m.get(k) {
        Some(v) => Some(v.clone().to_owned()),
        None => None,
    }
}

fn get_headers(row: &Row) -> StringRecord {
    row.iter().fold(StringRecord::new(), |mut acc,col| {
        if !col.config.exclude {
            acc.push_field(col.config.name.as_str());
        }
        acc
    })
}

fn set_context(row: &Row, lua: &Lua) {
    lua.context(|ctx| {
        let globals = ctx.globals();
        for col in row {
            let key = col.config.name.as_str();
            let value = col.value.clone().unwrap_or(String::new());
            globals.set(key, value).unwrap();
        };
    });
}

fn array_diff<T: PartialEq>(a: Vec<T>, b: Vec<T>) -> Vec<T> {
    let mut a = a;
    a.retain(|aa| !b.contains(aa));
    a
}

fn read_header_information<R: io::Read + 'static>(rdr: &mut csv::Reader<R>) -> StringRecord {
    rdr.headers().unwrap().iter().map(|x| x.to_snake_case()).collect()
}

fn read_config(file_name: &str) -> Config {
    let contents = fs::read_to_string(file_name)
        .expect("Cannot read config file");
    toml::from_str(contents.as_str()).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_header_information() {
        let mut expect = StringRecord::new();
        expect.push_field("one");
        expect.push_field("twenty_one");
        let mut rdr = csv::Reader::from_reader("One,TwentyOne".as_bytes());
        assert_eq!(expect, read_header_information(&mut rdr));
    }
}
