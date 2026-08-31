//! User-guide §16 + VHS tape workflow — all Tier-1 `rgctl-tests/ecommerce-*` projects.
//!
//! Each project run uses an isolated temp copy of the fixture (artifacts under `.rgctl/`).
//! Requires `jq` on PATH.
//!
//! ```bash
//! cargo test --test user_guide_scenarios
//! ```

mod rgctl_harness;
#[path = "support/user_guide_harness.rs"]
mod user_guide_harness;

use user_guide_harness::{run_full_workflow, PROJECTS};

#[test]
fn user_guide_workflow_all_ecommerce_projects() {
    for project in PROJECTS {
        run_full_workflow(project);
    }
}
