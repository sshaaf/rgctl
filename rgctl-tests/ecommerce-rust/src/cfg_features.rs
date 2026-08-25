//! CFG feature probes for rgctl expected-facts (Rust lowering coverage).

#![allow(dead_code)]

pub fn cfg_short_circuit(a: bool, b: bool) -> i32 {
    if a && b {
        1
    } else {
        0
    }
}

pub fn cfg_match_guard(x: i32) -> i32 {
    match x {
        n if n > 0 => n,
        _ => 0,
    }
}

pub fn cfg_try_op(x: Result<i32, ()>) -> Result<i32, ()> {
    let v = x?;
    Ok(v + 1)
}

pub fn cfg_for_in(xs: &[i32]) -> i32 {
    let mut total = 0;
    for v in xs {
        total += *v;
    }
    total
}

pub async fn cfg_await() -> i32 {
    let x = async { 1 }.await;
    x
}
