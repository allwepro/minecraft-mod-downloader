use std::collections::HashMap;
use std::sync::Arc;

pub struct ArgDef {
    pub short: String,
    pub long: String,
    pub description: String,
}

pub type SharedArgRegistry = Arc<ArgRegistry>;

pub struct ArgRegistry {
    values: HashMap<String, String>,
    definitions: Vec<ArgDef>,
}

impl ArgRegistry {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|v| v.as_str())
    }

    pub fn print_help(&self) {
        let cyan = "\x1b[36m";
        let green = "\x1b[32m";
        let yellow = "\x1b[33m";
        let bold = "\x1b[1m";
        let reset = "\x1b[0m";

        println!(
            "{bold}{green}👾 Flux Project{reset} - {bold}Minecraft Resource Management \
        Tool{reset}"
        );
        println!();
        println!("{yellow}Usage:{reset}");

        for def in &self.definitions {
            let short = &def.short;
            let long = &def.long;
            let description = &def.description;

            println!("  {cyan}-{short}{reset} {cyan}--{long:<15}{reset} {description}");
        }
        println!();
        println!("{yellow}Example:{reset}");
        println!("  flux-launcher --path \"/home/user/.minecraft\" --help");
    }
}

pub struct ArgRegistryBuilder {
    definitions: Vec<ArgDef>,
}

impl ArgRegistryBuilder {
    pub fn new() -> Self {
        Self {
            definitions: Vec::new(),
        }
    }

    pub fn add(&mut self, short: &str, long: &str, description: &str) {
        self.definitions.push(ArgDef {
            short: short.to_string(),
            long: long.to_string(),
            description: description.to_string(),
        });
    }

    pub fn build(self, args: Vec<String>) -> Arc<ArgRegistry> {
        let mut values = HashMap::new();
        let mut i = 0;
        let quote_chars: &[char] = &['"', '\'', '`'];

        while i < args.len() {
            let arg = &args[i];

            for def in &self.definitions {
                let s_flag = format!("-{}", def.short);
                let l_flag = format!("--{}", def.long);

                if arg == &s_flag || arg == &l_flag {
                    if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        let raw_val = &args[i + 1];
                        let val = raw_val.trim_matches(quote_chars).to_string();
                        values.insert(def.long.clone(), val);
                        i += 1;
                    } else {
                        values.insert(def.long.clone(), "true".to_string());
                    }
                    break;
                }
            }
            i += 1;
        }

        Arc::new(ArgRegistry {
            values,
            definitions: self.definitions,
        })
    }
}
