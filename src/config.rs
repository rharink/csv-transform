#[derive(Deserialize, PartialEq, Debug)]
pub struct Config {
    pub version: String,
    pub input: Input,
    pub output: Output,
    pub filter: Option<String>,
    columns: Vec<ColDef>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            version: "1".to_string(),
            input: Input::default(),
            output: Output::default(),
            filter: None,
            columns: vec![],
        }
    }
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct ColDef {
    name: String,
    func: Option<String>,
    exclude: Option<bool>,
}

impl ColDef {
    pub fn new(name: String) -> ColDef
    {
        ColDef {
            name,
            func: None,
            exclude: None,
        }
    }

    pub fn get_func(&self) -> &Option<String> {
        &self.func
    }

    pub fn get_name(&self) -> &str {
        &self.name.as_str()
    }

    pub fn get_exclude(&self) -> &Option<bool> {
        &self.exclude
    }
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct Input {
    pub trim: bool,
    pub quoting: bool,
    pub double_quote: bool,
    pub quote: Option<char>,
    pub escape: Option<char>,
    pub comment: Option<char>,
    pub delimiter: Option<char>,
    pub terminator: Option<char>,
}

impl Default for Input {
    fn default() -> Input {
        Input {
            trim: false,
            quoting: true,
            double_quote: true,
            quote: None,
            escape: None,
            comment: None,
            delimiter: None,
            terminator: None,
        }
    }
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct Output {
    pub header: bool,
    pub double_quote: bool,
    pub quote: Option<char>,
    pub escape: Option<char>,
    pub delimiter: Option<char>,
    pub terminator: Option<char>,
}

impl Default for Output {
    fn default() -> Output {
        Output {
            header: true,
            double_quote: true,
            quote: None,
            escape: None,
            delimiter: None,
            terminator: None
        }
    }
}

impl Config {
    pub fn new() -> Config {
        Config::default()
    }

    pub fn from_file(path: &str) -> Config {
        let contents = std::fs::read_to_string(path).expect("cannot read file");
        toml::from_str(contents.as_str()).expect("invalid toml")
    }

    pub fn get_headers(&self) -> Vec<String> {
        self.columns.iter().map(|col| col.name.clone()).collect()
    }

    pub fn get_column_definition(&self, name: &str) -> Option<&ColDef> {
        self.columns.iter().find(|c| {
            c.name == name
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default_config() {
        let expect = Config {
            version: "1".to_string(),
            input: Input {
                trim: false,
                quoting: true,
                double_quote: true,
                quote: None,
                escape: None,
                comment: None,
                delimiter: None,
                terminator: None,
            },
            output: Output {
                header: true,
                double_quote: true,
                quote: None,
                escape: None,
                delimiter: None,
                terminator: None
            },
            filter: None,
            columns: vec![],
        };

        assert_eq!(expect, Config::default());
    }
}