use std::fmt::Write;

pub trait Colored {
    // fn red(&self) -> String;
    // fn green(&self) -> String;
    fn yellow(&self) -> String;
    // fn bold(&self) -> String;
}

impl Colored for String {
    // fn red(&self) -> String {
    //     format!("\x1b[31;1m{self}\x1b[0m")
    // }
    //
    // fn green(&self) -> String {
    //     format!("\x1b[32;1m{self}\x1b[0m")
    // }

    fn yellow(&self) -> String {
        format!("\x1b[33;1m{self}\x1b[0m")
    }

    // fn bold(&self) -> String {
    //     format!("\x1b[1m{self}\x1b[0m")
    // }
}

impl Colored for &str {
    // fn red(&self) -> String {
    //     (*self).to_string().red()
    // }

    // fn green(&self) -> String {
    //     (*self).to_string().green()
    // }

    fn yellow(&self) -> String {
        (*self).to_string().yellow()
    }

    // fn bold(&self) -> String {
    //     (*self).to_string().bold()
    // }
}

pub(crate) fn mark_differences(original: &str, modified: &str) -> String {
    let mut result = String::new();
    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();

    let max_lines = original_lines.len().max(modified_lines.len());

    for i in 0..max_lines {
        let original_line = original_lines.get(i).unwrap_or(&"");
        let modified_line = modified_lines.get(i).unwrap_or(&"");

        if original_line == modified_line {
            result.push_str(original_line);
        } else {
            // Highlight differences in red
            let _ = write!(result, "{}", original_line.yellow());
        }
        result.push('\n'); // Add a newline for separation
    }

    result
}
