use super::WorkflowContext;
use anyhow::Result;
use handlebars::{Handlebars, Helper, Context, RenderContext, Output, HelperResult};
use serde_json::Value;
use std::collections::HashMap;

/// Template engine for workflow parameter resolution
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        
        // Register custom helpers
        handlebars.register_helper("json_path", Box::new(json_path_helper));
        handlebars.register_helper("base64_encode", Box::new(base64_encode_helper));
        handlebars.register_helper("base64_decode", Box::new(base64_decode_helper));
        handlebars.register_helper("regex_match", Box::new(regex_match_helper));
        handlebars.register_helper("regex_replace", Box::new(regex_replace_helper));
        handlebars.register_helper("format_date", Box::new(format_date_helper));
        handlebars.register_helper("uuid", Box::new(uuid_helper));
        handlebars.register_helper("env", Box::new(env_helper));
        
        Self { handlebars }
    }
    
    /// Resolve parameters using template engine
    pub fn resolve_parameters(
        &self,
        parameters: &HashMap<String, Value>,
        context: &WorkflowContext,
    ) -> Result<HashMap<String, Value>> {
        let mut resolved = HashMap::new();
        
        for (key, value) in parameters {
            let resolved_value = self.resolve_value(value, context)?;
            resolved.insert(key.clone(), resolved_value);
        }
        
        Ok(resolved)
    }
    
    /// Resolve a single value
    fn resolve_value(&self, value: &Value, context: &WorkflowContext) -> Result<Value> {
        match value {
            Value::String(s) => {
                if s.contains("{{") {
                    let template_data = self.build_template_data(context)?;
                    let rendered = self.handlebars.render_template(s, &template_data)?;
                    
                    // Try to parse as JSON if it looks like structured data
                    if rendered.starts_with('{') || rendered.starts_with('[') || rendered.starts_with('"') {
                        match serde_json::from_str(&rendered) {
                            Ok(parsed) => Ok(parsed),
                            Err(_) => Ok(Value::String(rendered)),
                        }
                    } else {
                        Ok(Value::String(rendered))
                    }
                } else {
                    Ok(value.clone())
                }
            }
            Value::Object(obj) => {
                let mut resolved_obj = serde_json::Map::new();
                for (k, v) in obj {
                    resolved_obj.insert(k.clone(), self.resolve_value(v, context)?);
                }
                Ok(Value::Object(resolved_obj))
            }
            Value::Array(arr) => {
                let resolved_arr: Result<Vec<_>> = arr.iter()
                    .map(|v| self.resolve_value(v, context))
                    .collect();
                Ok(Value::Array(resolved_arr?))
            }
            _ => Ok(value.clone()),
        }
    }
    
    /// Build template data from workflow context
    fn build_template_data(&self, context: &WorkflowContext) -> Result<Value> {
        let mut data = serde_json::Map::new();
        
        // Add inputs
        data.insert("inputs".to_string(), Value::Object(
            context.inputs.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        ));
        
        // Add step outputs
        data.insert("steps".to_string(), Value::Object(
            context.step_outputs.iter()
                .map(|(step_id, outputs)| {
                    (step_id.clone(), Value::Object(
                        outputs.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    ))
                })
                .collect()
        ));
        
        // Add variables
        data.insert("variables".to_string(), Value::Object(
            context.variables.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        ));
        
        // Add workflow metadata
        data.insert("workflow".to_string(), serde_json::json!({
            "id": context.workflow_id,
            "execution_id": context.execution_id,
            "start_time": context.start_time.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs()
        }));
        
        Ok(Value::Object(data))
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function for JSONPath-like access
fn json_path_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let path = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let data = h.param(1).map(|v| v.value()).unwrap_or(&Value::Null);
    
    let result = extract_json_path(data, path);
    out.write(&serde_json::to_string(&result).unwrap_or_default())?;
    Ok(())
}

/// Helper function for base64 encoding
fn base64_encode_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let input = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, input.as_bytes());
    out.write(&encoded)?;
    Ok(())
}

/// Helper function for base64 decoding
fn base64_decode_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let input = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, input) {
        Ok(decoded) => {
            if let Ok(decoded_str) = String::from_utf8(decoded) {
                out.write(&decoded_str)?;
            }
        }
        Err(_) => {
            out.write(input)?; // Return original if decode fails
        }
    }
    Ok(())
}

/// Helper function for regex matching
fn regex_match_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let pattern = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let text = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
    
    match regex::Regex::new(pattern) {
        Ok(re) => {
            let matches: Vec<String> = re.find_iter(text)
                .map(|m| m.as_str().to_string())
                .collect();
            out.write(&serde_json::to_string(&matches).unwrap_or_default())?;
        }
        Err(_) => {
            out.write("[]")?; // Return empty array if regex is invalid
        }
    }
    Ok(())
}

/// Helper function for regex replacement
fn regex_replace_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let pattern = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let replacement = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
    let text = h.param(2).and_then(|v| v.value().as_str()).unwrap_or("");
    
    match regex::Regex::new(pattern) {
        Ok(re) => {
            let result = re.replace_all(text, replacement);
            out.write(&result)?;
        }
        Err(_) => {
            out.write(text)?; // Return original if regex is invalid
        }
    }
    Ok(())
}

/// Helper function for date formatting
fn format_date_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let format = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("%Y-%m-%d %H:%M:%S");
    let timestamp = h.param(1).and_then(|v| v.value().as_u64());
    
    let datetime = if let Some(ts) = timestamp {
        chrono::DateTime::from_timestamp(ts as i64, 0)
    } else {
        Some(chrono::Utc::now())
    };
    
    if let Some(dt) = datetime {
        out.write(&dt.format(format).to_string())?;
    } else {
        out.write("")?;
    }
    Ok(())
}

/// Helper function for UUID generation
fn uuid_helper(
    _: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let uuid = uuid::Uuid::new_v4().to_string();
    out.write(&uuid)?;
    Ok(())
}

/// Helper function for environment variable access
fn env_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let var_name = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let default_value = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
    
    let value = std::env::var(var_name).unwrap_or_else(|_| default_value.to_string());
    out.write(&value)?;
    Ok(())
}

/// Extract value from JSON using simple path notation
fn extract_json_path(data: &Value, path: &str) -> Value {
    if path.is_empty() {
        return data.clone();
    }
    
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = data;
    
    for part in parts {
        match current {
            Value::Object(obj) => {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return Value::Null;
                }
            }
            Value::Array(arr) => {
                if let Ok(index) = part.parse::<usize>() {
                    if let Some(value) = arr.get(index) {
                        current = value;
                    } else {
                        return Value::Null;
                    }
                } else {
                    return Value::Null;
                }
            }
            _ => return Value::Null,
        }
    }
    
    current.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_template_engine_creation() {
        let _engine = TemplateEngine::new();
        // Template engine created successfully
    }
    
    #[test]
    fn test_json_path_extraction() {
        let data = serde_json::json!({
            "user": {
                "name": "John",
                "age": 30
            },
            "items": [1, 2, 3]
        });
        
        assert_eq!(extract_json_path(&data, "user.name"), Value::String("John".to_string()));
        assert_eq!(extract_json_path(&data, "user.age"), Value::Number(30.into()));
        assert_eq!(extract_json_path(&data, "items.0"), Value::Number(1.into()));
        assert_eq!(extract_json_path(&data, "nonexistent"), Value::Null);
    }
    
    #[tokio::test]
    async fn test_parameter_resolution() {
        let engine = TemplateEngine::new();
        let mut context = WorkflowContext::new(
            "test_workflow".to_string(),
            "exec_123".to_string(),
            HashMap::new(),
        );
        
        context.set_variable("test_var", serde_json::json!("test_value"));
        
        let mut parameters = HashMap::new();
        parameters.insert("static".to_string(), serde_json::json!("static_value"));
        parameters.insert("dynamic".to_string(), serde_json::json!("{{ variables.test_var }}"));
        
        let resolved = engine.resolve_parameters(&parameters, &context).unwrap();
        
        assert_eq!(resolved.get("static"), Some(&serde_json::json!("static_value")));
        assert_eq!(resolved.get("dynamic"), Some(&serde_json::json!("test_value")));
    }
}
