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

pub(crate) fn hamming_distance(vec1: &[u8], vec2: &[u8]) -> Option<usize> {
    // Check if the lengths of the two vectors are the same
    if vec1.len() != vec2.len() {
        return None; // Return None if lengths are different
    }

    let mut distance = 0;

    // Iterate through both vectors
    for (byte1, byte2) in vec1.iter().zip(vec2.iter()) {
        // Count differing bits using XOR and then count the bits set to 1
        distance += (byte1 ^ byte2).count_ones() as usize;
    }

    Some(distance)
}
