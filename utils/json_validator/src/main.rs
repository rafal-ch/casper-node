use jsonschema::JSONSchema;
use serde_json::Value;

fn main() {
    println!("cargo test please...");
}

pub type ValidationErrors = Vec<String>;

#[derive(Debug)]
pub enum JsonValidatorError {
    Other(String),
}

pub struct JsonValidator {
    schema: JSONSchema,
}

impl JsonValidator {
    pub fn try_from_schema(schema: &str) -> Result<Self, JsonValidatorError> {
        let schema = serde_json::from_str(schema)
            .map_err(|err| JsonValidatorError::Other(err.to_string()))?;
        let compiled = JSONSchema::compile(&schema)
            .map_err(|err| JsonValidatorError::Other(err.to_string()))?;
        Ok(Self { schema: compiled })
    }

    pub fn validate(&self, actual: &str) -> ValidationErrors {
        let actual: Result<Value, _> = serde_json::from_str(actual);
        if let Ok(actual) = actual {
            match self.schema.validate(&actual) {
                Ok(_) => vec![],
                Err(errors) => errors.into_iter().map(|err| err.to_string()).collect(),
            }
        } else {
            vec!["Provided 'actual' is not a correct JSON".to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
    const SCHEMA: &str = r#"
        
    {
     "description": "Validation of people",
     "positiveInteger": {
         "type": "integer",
         "minimum": 0,
         "maximum": 200
     },
     "type": "object",
     "properties": {
         "name": {
             "type": "string"
         },
         "surname": {
             "type": "string"
         },
         "age": {
             "type": "integer",
             "minimum": 0,
             "maximum": 150
         }
     },
     "required": ["surname"],
     "additionalProperties": false
 }

  "#;

    mod ok {
        use super::*;
        use crate::JsonValidator;

        #[test]
        fn perfect_match() {
            let actual = r#"
                    {
                        "name": "Agata",
                        "surname": "Beret",
                        "age": 99
                    }"#;

            let validator =
                JsonValidator::try_from_schema(SCHEMA).expect("Unable to create validator");
            let errors = validator.validate(actual);
            if !errors.is_empty() {
                dump_errors(errors);
                assert!(false);
            } else {
                assert!(true)
            }
        }

        #[test]
        fn perfect_match_different_order() {
            let actual = r#"
                    {
                        "age": 99,
                        "surname": "Beret",
                        "name": "Agata"
                    }"#;

            let validator =
                JsonValidator::try_from_schema(SCHEMA).expect("Unable to create validator");
            let errors = validator.validate(actual);
            if !errors.is_empty() {
                dump_errors(errors);
                assert!(false);
            } else {
                assert!(true)
            }
        }
    }

    mod err {
        use super::*;
        use crate::JsonValidator;

        #[test]
        fn too_old() {
            let actual = r#"
                    {
                        "age": 1326,
                        "surname": "Beret",
                        "name": "Agata"
                    }"#;

            let validator =
                JsonValidator::try_from_schema(SCHEMA).expect("Unable to create validator");
            let errors = validator.validate(actual);
            if !errors.is_empty() {
                dump_errors(errors);
                assert!(true);
            } else {
                assert!(false)
            }
        }

        #[test]
        fn missing_required_field() {
            let actual = r#"
                    {
                        "name": "Agata",
                        "age": 99
                    }"#;

            let validator =
                JsonValidator::try_from_schema(SCHEMA).expect("Unable to create validator");
            let errors = validator.validate(actual);
            if !errors.is_empty() {
                dump_errors(errors);
                assert!(true);
            } else {
                assert!(false)
            }
        }

        #[test]
        fn superfluous_field() {
            let actual = r#"
                    {
                        "name": "Agata",
                        "surname": "Beret",
                        "age": 99,
                        "should_not": "be here"
                    }"#;

            let validator =
                JsonValidator::try_from_schema(SCHEMA).expect("Unable to create validator");
            let errors = validator.validate(actual);
            if !errors.is_empty() {
                dump_errors(errors);
                assert!(true);
            } else {
                assert!(false)
            }
        }

        #[test]
        fn incorrect_json_passed_for_validation() {
            let actual = "this is NOT a JSON";

            let validator =
                JsonValidator::try_from_schema(SCHEMA).expect("Unable to create validator");
            let errors = validator.validate(actual);
            if !errors.is_empty() {
                dump_errors(errors);
                assert!(true);
            } else {
                assert!(false)
            }
        }

        #[test]
        fn incorrect_schema() {
            let validator = JsonValidator::try_from_schema("this is not a correct schema");
            assert!(validator.is_err());
        }
    }

    fn dump_errors<'a>(errors: impl IntoIterator<Item = String>) {
        errors
            .into_iter()
            .for_each(|error| println!("\tERROR: {}", error));
    }
}
