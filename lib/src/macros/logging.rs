/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

#[macro_export]
macro_rules! log_elapsed {
    ($func_name:expr,$msg:expr) => {{
        if cfg!(debug_assertions) {
            use std::time::Instant;
            let start = Instant::now();
            let result = $func_name();
            let elapsed = start.elapsed();
            if elapsed.as_secs() > 2 {
                println!("Function {} : Elapsed :{:?}", $msg, elapsed);
            }
            result
        } else {
            $func_name()
        }
    }};
    ($func_name:expr) => {{
        if cfg!(debug_assertions) {
            use std::time::Instant;
            let start = Instant::now();
            let fn_return = $func_name;
            let elapsed = start.elapsed();
            if elapsed.as_secs() > 2 {
                println!("Function {}: Elapsed {:?}", stringify!($func_name), elapsed);
            }
            fn_return
        } else {
            $func_name
        }
    }};
}
