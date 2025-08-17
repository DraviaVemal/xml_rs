/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

/// Macro for measuring and logging the execution time of a function or expression.
///
/// This macro exists in two forms:
/// 1. With a message parameter: `log_elapsed!(function_call, "Message")`
/// 2. Without a message: `log_elapsed!(function_call)`
///
/// In both cases, the macro:
/// - Only performs timing in debug builds
/// - Returns the result of the provided function/expression
/// - Logs elapsed time if it exceeds 2 seconds
///
/// # Arguments
/// * `$func_name` - The function or expression to time.
/// * `$msg` - (Optional) A message to include in the log.
///
/// # Examples
/// ```
/// let result = log_elapsed!(expensive_function(), "Expensive operation");
/// ```
#[macro_export]
macro_rules! log_elapsed {
    // Form with custom message
    ($func_name:expr,$msg:expr) => {{
        if cfg!(debug_assertions) {
            // Only measure time in debug builds
            use std::time::Instant;
            let start = Instant::now();
            let result = $func_name();
            let elapsed = start.elapsed();
            
            // Only log if the operation took more than 2 seconds
            if elapsed.as_secs() > 2 {
                println!("Function {} : Elapsed :{:?}", $msg, elapsed);
            }
            result
        } else {
            // In release mode, just execute the function without timing
            $func_name()
        }
    }};
    
    // Form without custom message
    ($func_name:expr) => {{
        if cfg!(debug_assertions) {
            // Only measure time in debug builds
            use std::time::Instant;
            let start = Instant::now();
            let fn_return = $func_name;
            let elapsed = start.elapsed();
            
            // Only log if the operation took more than 2 seconds
            if elapsed.as_secs() > 2 {
                println!("Function {}: Elapsed {:?}", stringify!($func_name), elapsed);
            }
            fn_return
        } else {
            // In release mode, just return the function without timing
            $func_name
        }
    }};
}
