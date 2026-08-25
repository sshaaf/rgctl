//! May-alias heuristics for hybrid CPG P3 T2 (on-demand only).
//!
//! Conservative name-based rules — not a full points-to analysis:
//! - field access shares a base (`order` ↔ `order.status`)
//! - simple copy assignments `a = b` (identifier LHS/RHS) union names

use crate::cfg::ControlFlowGraph;
use std::collections::{HashMap, HashSet};

struct VarIntern {
    names: Vec<String>,
    ids: HashMap<String, u32>,
}

impl VarIntern {
    fn intern(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let id = self.names.len() as u32;
        self.names.push(name.to_string());
        self.ids.insert(name.to_string(), id);
        id
    }
}

fn uf_find(parent: &mut [u32], x: u32) -> u32 {
    if parent[x as usize] == x {
        return x;
    }
    let root = uf_find(parent, parent[x as usize]);
    parent[x as usize] = root;
    root
}

fn uf_union(parent: &mut [u32], a: u32, b: u32) {
    let ra = uf_find(parent, a);
    let rb = uf_find(parent, b);
    if ra != rb {
        parent[ra as usize] = rb;
    }
}

fn grow_parent(parent: &mut Vec<u32>, id: u32) {
    let need = (id + 1) as usize;
    if parent.len() < need {
        let old = parent.len();
        parent.resize(need, 0);
        for i in old..need {
            parent[i] = i as u32;
        }
    }
}

fn union_names(intern: &mut VarIntern, parent: &mut Vec<u32>, a: &str, b: &str) {
    let a_id = intern.intern(a);
    let b_id = intern.intern(b);
    grow_parent(parent, a_id.max(b_id));
    uf_union(parent, a_id, b_id);
}

/// Expand `seed` to a may-alias name set for slicing / flows.
pub fn may_alias_names(cfg: &ControlFlowGraph, seed: &str) -> HashSet<String> {
    let mut intern = VarIntern {
        names: Vec::new(),
        ids: HashMap::new(),
    };
    let mut parent: Vec<u32> = Vec::new();

    let seed_id = intern.intern(seed);
    grow_parent(&mut parent, seed_id);
    if let Some((base, _)) = seed.split_once('.') {
        union_names(&mut intern, &mut parent, seed, base);
    }

    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            for d in &stmt.defined_vars {
                grow_parent(&mut parent, intern.intern(d));
                if let Some((base, _)) = d.split_once('.') {
                    union_names(&mut intern, &mut parent, d, base);
                }
            }
            for u in &stmt.used_vars {
                grow_parent(&mut parent, intern.intern(u));
                if let Some((base, _)) = u.split_once('.') {
                    union_names(&mut intern, &mut parent, u, base);
                }
            }
            if stmt.defined_vars.len() == 1 && stmt.used_vars.len() == 1 {
                let d = stmt.defined_vars.iter().next().unwrap();
                let u = stmt.used_vars.iter().next().unwrap();
                if !d.contains('.') && !u.contains('.') {
                    union_names(&mut intern, &mut parent, d, u);
                }
            }
        }
    }

    let seed_root = uf_find(&mut parent, seed_id);
    intern
        .names
        .iter()
        .enumerate()
        .filter_map(|(i, name)| {
            if uf_find(&mut parent, i as u32) == seed_root {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_for_function;

    #[test]
    fn field_and_copy_alias() {
        let code = r#"
public class C {
    void m(OrderDTO order) {
        OrderDTO other = order;
        other.status = "X";
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "m").unwrap();
        let set = may_alias_names(&cfg, "order");
        assert!(set.contains("order"));
        assert!(set.contains("other"), "copy alias missing: set={set:?}");
        assert!(
            set.contains("other.status"),
            "field via alias missing: set={set:?}"
        );
    }
}
