/// Converts an identifier in any common casing (snake_case, kebab-case, PascalCase,
/// camelCase, space separated...) to PascalCase. Idempotent on input that is already
/// PascalCase, since callers pass user-typed TOML strings straight into `quote!`.
pub fn to_pascal_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut capitalize_next = true;

    for ch in input.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            capitalize_next = true;
            continue;
        }
        if capitalize_next {
            out.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            out.push(ch);
        }
    }

    out
}

/// Converts an identifier in any common casing to snake_case, for use in field,
/// module, and file names.
pub fn to_snake_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_was_lower_or_digit = false;

    for ch in input.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            prev_was_lower_or_digit = false;
            continue;
        }
        if ch.is_uppercase() {
            if prev_was_lower_or_digit {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
            prev_was_lower_or_digit = false;
        } else {
            out.push(ch);
            prev_was_lower_or_digit = ch.is_lowercase() || ch.is_numeric();
        }
    }

    out
}

/// snake_cases `name` and appends `_{suffix}` unless it already ends with that
/// suffix, avoiding stutter like `sensor_backend_backend`.
pub fn snake_with_suffix(name: &str, suffix: &str) -> String {
    let snake = to_snake_case(name);
    let marker = format!("_{}", suffix);
    if snake == suffix || snake.ends_with(&marker) {
        snake
    } else {
        format!("{}{}", snake, marker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_is_idempotent() {
        assert_eq!(to_pascal_case("SensorBackend"), "SensorBackend");
    }

    #[test]
    fn pascal_case_from_snake() {
        assert_eq!(to_pascal_case("sensor_backend"), "SensorBackend");
    }

    #[test]
    fn snake_case_from_pascal() {
        assert_eq!(to_snake_case("SensorBackend"), "sensor_backend");
    }

    #[test]
    fn snake_case_is_idempotent() {
        assert_eq!(to_snake_case("sensor_backend"), "sensor_backend");
    }

    #[test]
    fn snake_with_suffix_avoids_stutter() {
        assert_eq!(snake_with_suffix("SensorBackend", "backend"), "sensor_backend");
        assert_eq!(snake_with_suffix("Sensor", "backend"), "sensor_backend");
    }
}
