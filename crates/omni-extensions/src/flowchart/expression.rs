use std::collections::HashMap;

use super::error::{FlowchartError, Result};

/// Execution context data available to expressions.
pub struct ExpressionContext<'a> {
    /// Tool parameters from the trigger.
    pub params: &'a serde_json::Value,
    /// Outputs from previously executed nodes, keyed by node ID.
    pub node_outputs: &'a HashMap<String, serde_json::Value>,
    /// Named variables set by SetVariable nodes.
    pub variables: &'a HashMap<String, serde_json::Value>,
}

/// Evaluate a JSONPath-like expression against the context.
///
/// Supported paths:
/// - `$.params.field` -- access tool parameters
/// - `$.nodes.{node_id}.field` -- access a specific node's output
/// - `$var.name` -- access a named variable
/// - `$.field.nested[0].deep` -- nested access with array indexing
pub fn evaluate_path(path: &str, ctx: &ExpressionContext) -> Result<serde_json::Value> {
    let path = path.trim();

    if !path.starts_with('$') {
        // Literal string or number
        return parse_literal(path);
    }

    let segments = parse_path_segments(path)?;

    if segments.is_empty() {
        return Err(FlowchartError::Expression("Empty path".to_string()));
    }

    // Determine root value
    let (root, rest) = match segments[0].as_str() {
        "params" => (ctx.params.clone(), &segments[1..]),
        "nodes" => {
            if segments.len() < 2 {
                return Err(FlowchartError::Expression(
                    "$.nodes requires a node ID: $.nodes.{node_id}.field".to_string(),
                ));
            }
            let node_id = &segments[1];
            let val = ctx
                .node_outputs
                .get(node_id.as_str())
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            (val, &segments[2..])
        }
        "var" => {
            if segments.len() < 2 {
                return Err(FlowchartError::Expression(
                    "$var requires a variable name: $var.name".to_string(),
                ));
            }
            let var_name = &segments[1];
            let val = ctx
                .variables
                .get(var_name.as_str())
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            (val, &segments[2..])
        }
        other => {
            return Err(FlowchartError::Expression(format!(
                "Unknown root '${other}'. Use $.params, $.nodes, or $var"
            )));
        }
    };

    // Navigate the rest of the path
    navigate_value(&root, rest)
}

/// Interpolate a template string, replacing `{{expression}}` with evaluated values.
///
/// Example: `"Hello {{$.params.name}}, you have {{$.nodes.count_node.total}} items"`
pub fn evaluate_template(template: &str, ctx: &ExpressionContext) -> Result<String> {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let mut expr = String::new();
            let mut depth = 1;
            while let Some(c) = chars.next() {
                if c == '{' && chars.peek() == Some(&'{') {
                    depth += 1;
                    expr.push(c);
                    chars.next();
                    expr.push('{');
                } else if c == '}' && chars.peek() == Some(&'}') {
                    depth -= 1;
                    if depth == 0 {
                        chars.next(); // consume second '}'
                        break;
                    }
                    expr.push(c);
                    chars.next();
                    expr.push('}');
                } else {
                    expr.push(c);
                }
            }
            let val = evaluate_path(expr.trim(), ctx)?;
            result.push_str(&value_to_string(&val));
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Evaluate a condition expression, returning true or false.
///
/// Supported forms:
/// - `$.params.status == 200`
/// - `$.params.name contains "hello"`
/// - `$.params.items exists`
/// - `$.params.count > 5 && $.params.active == true`
pub fn evaluate_condition(expression: &str, ctx: &ExpressionContext) -> Result<bool> {
    let expr = expression.trim();

    // Handle boolean operators: split on && and ||
    if let Some(pos) = find_top_level_operator(expr, "||") {
        let left = evaluate_condition(&expr[..pos], ctx)?;
        let right = evaluate_condition(&expr[pos + 2..], ctx)?;
        return Ok(left || right);
    }
    if let Some(pos) = find_top_level_operator(expr, "&&") {
        let left = evaluate_condition(&expr[..pos], ctx)?;
        let right = evaluate_condition(&expr[pos + 2..], ctx)?;
        return Ok(left && right);
    }

    // Handle negation
    if let Some(inner) = expr.strip_prefix('!') {
        let val = evaluate_condition(inner.trim(), ctx)?;
        return Ok(!val);
    }

    // Try unary operators first (e.g., `$.params.x exists`)
    for (op, unary_fn) in UNARY_OPERATORS {
        if let Some(path) = expr.strip_suffix(op) {
            let val = evaluate_path(path.trim(), ctx)?;
            return Ok(unary_fn(&val));
        }
    }

    // Try binary operators
    for (op, binary_fn) in BINARY_OPERATORS {
        if let Some(pos) = find_operator_position(expr, op) {
            let left_expr = expr[..pos].trim();
            let right_expr = expr[pos + op.len()..].trim();
            let left = evaluate_path(left_expr, ctx)?;
            let right = evaluate_path(right_expr, ctx)?;
            return Ok(binary_fn(&left, &right));
        }
    }

    // Single value: truthy check
    let val = evaluate_path(expr, ctx)?;
    Ok(is_truthy(&val))
}

// ── Internal helpers ────────────────────────────────────────────────

const BINARY_OPERATORS: &[(&str, fn(&serde_json::Value, &serde_json::Value) -> bool)] = &[
    (" == ", |a, b| values_equal(a, b)),
    (" != ", |a, b| !values_equal(a, b)),
    (" >= ", |a, b| compare_values(a, b) >= Some(std::cmp::Ordering::Equal)),
    (" <= ", |a, b| compare_values(a, b) <= Some(std::cmp::Ordering::Equal)),
    (" > ", |a, b| compare_values(a, b) == Some(std::cmp::Ordering::Greater)),
    (" < ", |a, b| compare_values(a, b) == Some(std::cmp::Ordering::Less)),
    (" contains ", |a, b| {
        let a_str = value_to_string(a);
        let b_str = value_to_string(b);
        a_str.contains(&b_str)
    }),
    (" starts_with ", |a, b| {
        let a_str = value_to_string(a);
        let b_str = value_to_string(b);
        a_str.starts_with(&b_str)
    }),
    (" matches ", |a, b| {
        let a_str = value_to_string(a);
        let pattern = value_to_string(b);
        // SECURITY: Rust's `regex` crate uses Thompson NFA / lazy DFA internally,
        // which guarantees O(n*m) matching -- immune to catastrophic backtracking
        // (ReDoS). The 1MB size limit prevents excessive memory from very large
        // compiled patterns. No additional ReDoS protection needed.
        regex::RegexBuilder::new(&pattern)
            .size_limit(1 << 20) // 1 MB compiled size limit
            .build()
            .map(|r| r.is_match(&a_str))
            .unwrap_or(false)
    }),
];

const UNARY_OPERATORS: &[(&str, fn(&serde_json::Value) -> bool)] = &[
    (" exists", |v| !v.is_null()),
    (" is_null", |v| v.is_null()),
    (" is_string", |v| v.is_string()),
    (" is_number", |v| v.is_number()),
    (" is_array", |v| v.is_array()),
    (" is_object", |v| v.is_object()),
];

fn parse_literal(s: &str) -> Result<serde_json::Value> {
    let s = s.trim();

    // Try quoted string -- handle escape sequences (\" \' \\)
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        let inner = &s[1..s.len() - 1];
        let unescaped = inner
            .replace("\\\"", "\"")
            .replace("\\'", "'")
            .replace("\\\\", "\\")
            .replace("\\n", "\n")
            .replace("\\t", "\t");
        return Ok(serde_json::Value::String(unescaped));
    }

    // Try boolean
    if s == "true" {
        return Ok(serde_json::Value::Bool(true));
    }
    if s == "false" {
        return Ok(serde_json::Value::Bool(false));
    }

    // Try null
    if s == "null" {
        return Ok(serde_json::Value::Null);
    }

    // Try number
    if let Ok(n) = s.parse::<i64>() {
        return Ok(serde_json::json!(n));
    }
    if let Ok(n) = s.parse::<f64>() {
        return Ok(serde_json::json!(n));
    }

    // Treat as bare string
    Ok(serde_json::Value::String(s.to_string()))
}

/// Parse `$.foo.bar[0].baz` into segments `["foo", "bar", "[0]", "baz"]`.
fn parse_path_segments(path: &str) -> Result<Vec<String>> {
    // Strip leading `$` and optional `.`
    let path = path.strip_prefix('$').unwrap_or(path);
    let path = path.strip_prefix('.').unwrap_or(path);

    if path.is_empty() {
        return Ok(Vec::new());
    }

    let mut segments = Vec::new();
    let mut current = String::new();

    for ch in path.chars() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
                current.push('[');
            }
            ']' => {
                current.push(']');
                segments.push(std::mem::take(&mut current));
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }

    Ok(segments)
}

/// Navigate into a JSON value using path segments.
fn navigate_value(value: &serde_json::Value, segments: &[String]) -> Result<serde_json::Value> {
    let mut current = value.clone();
    for seg in segments {
        if seg.starts_with('[') && seg.ends_with(']') {
            // Array index
            let idx_str = &seg[1..seg.len() - 1];
            let idx: usize = idx_str
                .parse()
                .map_err(|_| FlowchartError::Expression(format!("Invalid array index: {seg}")))?;
            current = current
                .as_array()
                .and_then(|arr| arr.get(idx))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        } else {
            // Object field
            current = current
                .as_object()
                .and_then(|obj| obj.get(seg.as_str()))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        }
    }
    Ok(current)
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn is_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

fn values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    // Allow number-to-number comparisons across types
    if let (Some(a_num), Some(b_num)) = (a.as_f64(), b.as_f64()) {
        return (a_num - b_num).abs() < f64::EPSILON;
    }
    a == b
}

fn compare_values(
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> Option<std::cmp::Ordering> {
    if let (Some(a_num), Some(b_num)) = (a.as_f64(), b.as_f64()) {
        return a_num.partial_cmp(&b_num);
    }
    if let (Some(a_str), Some(b_str)) = (a.as_str(), b.as_str()) {
        return Some(a_str.cmp(b_str));
    }
    None
}

fn find_operator_position(expr: &str, op: &str) -> Option<usize> {
    // Find operator, avoiding matches inside quoted strings (handles \" escapes)
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_quotes {
            if ch == '\\' && i + 1 < bytes.len() {
                i += 2; // skip escaped character
                continue;
            }
            if ch == quote_char {
                in_quotes = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
            i += 1;
            continue;
        }
        if bytes[i..].starts_with(op_bytes) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_top_level_operator(expr: &str, op: &str) -> Option<usize> {
    // Find && or || outside quotes and parentheses (handles \" escapes)
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let mut paren_depth = 0u32;
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_quotes {
            if ch == '\\' && i + 1 < bytes.len() {
                i += 2; // skip escaped character
                continue;
            }
            if ch == quote_char {
                in_quotes = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
            i += 1;
            continue;
        }
        if ch == '(' {
            paren_depth += 1;
            i += 1;
            continue;
        }
        if ch == ')' {
            paren_depth = paren_depth.saturating_sub(1);
            i += 1;
            continue;
        }
        if paren_depth == 0 && bytes[i..].starts_with(op_bytes) {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(
        params: serde_json::Value,
        nodes: HashMap<String, serde_json::Value>,
        vars: HashMap<String, serde_json::Value>,
    ) -> (
        serde_json::Value,
        HashMap<String, serde_json::Value>,
        HashMap<String, serde_json::Value>,
    ) {
        (params, nodes, vars)
    }

    fn ctx_ref(
        data: &(
            serde_json::Value,
            HashMap<String, serde_json::Value>,
            HashMap<String, serde_json::Value>,
        ),
    ) -> ExpressionContext<'_> {
        ExpressionContext {
            params: &data.0,
            node_outputs: &data.1,
            variables: &data.2,
        }
    }

    #[test]
    fn test_path_params_simple() {
        let data = make_ctx(
            serde_json::json!({"name": "Alice", "age": 30}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        let val = evaluate_path("$.params.name", &ctx).unwrap();
        assert_eq!(val, serde_json::json!("Alice"));
    }

    #[test]
    fn test_path_params_nested() {
        let data = make_ctx(
            serde_json::json!({"user": {"name": "Bob", "scores": [10, 20, 30]}}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        let val = evaluate_path("$.params.user.name", &ctx).unwrap();
        assert_eq!(val, serde_json::json!("Bob"));

        let val = evaluate_path("$.params.user.scores[1]", &ctx).unwrap();
        assert_eq!(val, serde_json::json!(20));
    }

    #[test]
    fn test_path_node_output() {
        let mut nodes = HashMap::new();
        nodes.insert(
            "http_1".to_string(),
            serde_json::json!({"status": 200, "body": {"data": "hello"}}),
        );
        let data = make_ctx(serde_json::json!({}), nodes, HashMap::new());
        let ctx = ctx_ref(&data);

        let val = evaluate_path("$.nodes.http_1.status", &ctx).unwrap();
        assert_eq!(val, serde_json::json!(200));

        let val = evaluate_path("$.nodes.http_1.body.data", &ctx).unwrap();
        assert_eq!(val, serde_json::json!("hello"));
    }

    #[test]
    fn test_path_variable() {
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), serde_json::json!(42));
        let data = make_ctx(serde_json::json!({}), HashMap::new(), vars);
        let ctx = ctx_ref(&data);

        let val = evaluate_path("$var.count", &ctx).unwrap();
        assert_eq!(val, serde_json::json!(42));
    }

    #[test]
    fn test_path_missing_returns_null() {
        let data = make_ctx(serde_json::json!({}), HashMap::new(), HashMap::new());
        let ctx = ctx_ref(&data);
        let val = evaluate_path("$.params.nonexistent", &ctx).unwrap();
        assert_eq!(val, serde_json::Value::Null);
    }

    #[test]
    fn test_template_simple() {
        let data = make_ctx(
            serde_json::json!({"name": "World"}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        let result = evaluate_template("Hello {{$.params.name}}!", &ctx).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_template_multiple() {
        let data = make_ctx(
            serde_json::json!({"first": "John", "last": "Doe"}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        let result =
            evaluate_template("{{$.params.first}} {{$.params.last}}", &ctx).unwrap();
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn test_template_no_placeholders() {
        let data = make_ctx(serde_json::json!({}), HashMap::new(), HashMap::new());
        let ctx = ctx_ref(&data);
        let result = evaluate_template("Just plain text", &ctx).unwrap();
        assert_eq!(result, "Just plain text");
    }

    #[test]
    fn test_condition_equals() {
        let data = make_ctx(
            serde_json::json!({"status": 200}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.status == 200", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.status == 404", &ctx).unwrap());
    }

    #[test]
    fn test_condition_not_equals() {
        let data = make_ctx(
            serde_json::json!({"status": 200}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.status != 404", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.status != 200", &ctx).unwrap());
    }

    #[test]
    fn test_condition_comparisons() {
        let data = make_ctx(
            serde_json::json!({"count": 10}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.count > 5", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.count > 15", &ctx).unwrap());
        assert!(evaluate_condition("$.params.count < 20", &ctx).unwrap());
        assert!(evaluate_condition("$.params.count >= 10", &ctx).unwrap());
        assert!(evaluate_condition("$.params.count <= 10", &ctx).unwrap());
    }

    #[test]
    fn test_condition_contains() {
        let data = make_ctx(
            serde_json::json!({"text": "hello world"}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.text contains \"hello\"", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.text contains \"goodbye\"", &ctx).unwrap());
    }

    #[test]
    fn test_condition_starts_with() {
        let data = make_ctx(
            serde_json::json!({"text": "hello world"}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.text starts_with \"hello\"", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.text starts_with \"world\"", &ctx).unwrap());
    }

    #[test]
    fn test_condition_exists() {
        let data = make_ctx(
            serde_json::json!({"name": "Alice"}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.name exists", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.missing exists", &ctx).unwrap());
    }

    #[test]
    fn test_condition_boolean_and() {
        let data = make_ctx(
            serde_json::json!({"a": 1, "b": 2}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.a == 1 && $.params.b == 2", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.a == 1 && $.params.b == 3", &ctx).unwrap());
    }

    #[test]
    fn test_condition_boolean_or() {
        let data = make_ctx(
            serde_json::json!({"a": 1, "b": 2}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.a == 1 || $.params.b == 3", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.a == 5 || $.params.b == 3", &ctx).unwrap());
    }

    #[test]
    fn test_condition_negation() {
        let data = make_ctx(
            serde_json::json!({"active": false}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("!$.params.active", &ctx).unwrap());
    }

    #[test]
    fn test_condition_truthy() {
        let data = make_ctx(
            serde_json::json!({"name": "Alice", "count": 0, "empty": ""}),
            HashMap::new(),
            HashMap::new(),
        );
        let ctx = ctx_ref(&data);
        assert!(evaluate_condition("$.params.name", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.count", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.empty", &ctx).unwrap());
        assert!(!evaluate_condition("$.params.missing", &ctx).unwrap());
    }

    #[test]
    fn test_parse_literal_string() {
        assert_eq!(
            parse_literal("\"hello\"").unwrap(),
            serde_json::json!("hello")
        );
        assert_eq!(
            parse_literal("'world'").unwrap(),
            serde_json::json!("world")
        );
    }

    #[test]
    fn test_parse_literal_numbers() {
        assert_eq!(parse_literal("42").unwrap(), serde_json::json!(42));
        assert_eq!(parse_literal("3.14").unwrap(), serde_json::json!(3.14));
    }

    #[test]
    fn test_parse_literal_booleans() {
        assert_eq!(parse_literal("true").unwrap(), serde_json::json!(true));
        assert_eq!(parse_literal("false").unwrap(), serde_json::json!(false));
    }

    #[test]
    fn test_parse_literal_null() {
        assert_eq!(parse_literal("null").unwrap(), serde_json::Value::Null);
    }
}
