use std::collections::HashMap;
use regex::Regex;

pub struct VariableManager {
    variables: HashMap<String, String>,
}

impl VariableManager {
    pub fn new() -> Self {
        Self { variables: HashMap::new() }
    }

    pub fn set_variable(&mut self, variable_name: String, variable_value: String) {
        self.variables.insert(variable_name, variable_value);
    }

    pub fn get_variable(&self, variable_name: String) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(var) = self.variables.get(&variable_name) {
            Ok(var.clone())
        } else {
            Err(format!("Could not find variable: '{}'", variable_name).into())
        }
    }

    /// Parses variables in an input string, using the {{VARIABLE}} syntax
    pub fn resolve_variables(&self, input: &String) -> String {
        let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();

        re.replace_all(input, |caps: &regex::Captures| {
            let var_name = &caps[1];
            if let Some(value) = self.variables.get(var_name) {
                value.clone()
            } else {
                caps[0].to_string() // keep the original {{var}} if not found
            }
        }).to_string()
    }
}
