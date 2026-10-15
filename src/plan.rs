//! Plans: a reviewed batch of trash / move operations.
//!
//! Anyone with the token (the web UI, an AI over MCP) can propose a plan; it is
//! validated up front and shown with sizes, errors and warnings. Only an
//! explicit approve runs it: pending → approved (running) → done | failed,
//! or pending → rejected.

use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Trash,
    Move { volume: PathBuf, link: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Op {
    pub path: PathBuf,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct OpState {
    pub op: Op,
    pub size: u64,
    pub dest: Option<PathBuf>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// Set once the plan has run: Ok(message) or Err(reason).
    pub result: Option<Result<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Pending,
    Approved,
    Done,
    Failed,
    Rejected,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Approved => "approved",
            Status::Done => "done",
            Status::Failed => "failed",
            Status::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub id: u64,
    pub note: String,
    /// Who proposed it: "web", "mcp", ...
    pub origin: String,
    pub created: u64,
    pub status: Status,
    pub ops: Vec<OpState>,
}

impl Plan {
    pub fn total_bytes(&self) -> u64 {
        self.ops.iter().map(|o| o.size).sum()
    }

    pub fn valid(&self) -> bool {
        self.ops.iter().all(|o| o.errors.is_empty())
    }

    pub fn to_json(&self) -> Value {
        let ops: Vec<Value> = self
            .ops
            .iter()
            .map(|o| {
                let (action, volume, link) = match &o.op.action {
                    Action::Trash => ("trash", None, None),
                    Action::Move { volume, link } => {
                        ("move", Some(volume.display().to_string()), Some(*link))
                    }
                };
                json!({
                    "path": o.op.path.display().to_string(),
                    "action": action,
                    "volume": volume,
                    "link": link,
                    "size": o.size,
                    "dest": o.dest.as_ref().map(|d| d.display().to_string()),
                    "errors": o.errors,
                    "warnings": o.warnings,
                    "result": o.result.as_ref().map(|r| match r {
                        Ok(m) => json!({"ok": true, "message": m}),
                        Err(e) => json!({"ok": false, "message": e}),
                    }),
                })
            })
            .collect();
        json!({
            "id": self.id,
            "note": self.note,
            "origin": self.origin,
            "created": self.created,
            "status": self.status.as_str(),
            "total_bytes": self.total_bytes(),
            "valid": self.valid(),
            "ops": ops,
        })
    }
}

/// Parses `[{"action":"trash"|"move","path":…,"volume":…,"link":bool}]`.
pub fn parse_ops(v: &Value) -> Result<Vec<Op>, String> {
    let arr = v.as_array().ok_or("ops must be an array")?;
    if arr.is_empty() {
        return Err("ops is empty".into());
    }
    arr.iter()
        .map(|o| {
            let path = o["path"].as_str().ok_or("each op needs a string path")?;
            let action = match o["action"].as_str() {
                Some("trash") => Action::Trash,
                Some("move") => Action::Move {
                    volume: o["volume"].as_str().ok_or("move ops need a volume")?.into(),
                    link: o["link"].as_bool().unwrap_or(false),
                },
                _ => return Err("action must be \"trash\" or \"move\"".to_string()),
            };
            Ok(Op {
                path: path.into(),
                action,
            })
        })
        .collect()
}

/// Checks one op against the filesystem (sizes, rules, free space).
pub fn validate(op: &Op) -> OpState {
    let mut st = OpState {
        op: op.clone(),
        size: 0,
        dest: None,
        errors: vec![],
        warnings: vec![],
        result: None,
    };
    match &op.action {
        Action::Trash => match crate::relocate::normalize(&op.path) {
            Err(e) => st.errors.push(e),
            Ok(p) => {
                if let Some(r) =
                    crate::relocate::protected_reason(&p, crate::relocate::home().as_deref())
                {
                    st.errors.push(r);
                } else if std::fs::symlink_metadata(&p).is_err() {
                    st.errors.push(format!("{} does not exist", p.display()));
                } else {
                    st.size = crate::relocate::measure(&p, false).bytes;
                }
            }
        },
        Action::Move { volume, .. } => match crate::relocate::inspect(&op.path, volume) {
            Err(e) => st.errors.push(e),
            Ok((c, m)) => {
                st.size = m.bytes;
                st.dest = Some(c.dest);
                st.errors = c.errors;
                st.warnings = c.warnings;
            }
        },
    }
    st
}

/// Runs one op for real. Re-checks the rules first: the disk may have changed
/// since the plan was proposed.
pub fn execute(op: &Op) -> Result<String, String> {
    let st = validate(op);
    if !st.errors.is_empty() {
        return Err(st.errors.join("; "));
    }
    match &op.action {
        Action::Trash => {
            crate::platform::trash::move_to_trash(&op.path)?;
            Ok("moved to Trash".into())
        }
        Action::Move { volume, link } => {
            let o = crate::relocate::relocate(&op.path, volume, *link)?;
            Ok(format!(
                "moved {} files ({} bytes) to {}{}",
                o.files,
                o.bytes,
                o.dest.display(),
                if o.linked { ", link left behind" } else { "" }
            ))
        }
    }
}

#[derive(Default)]
pub struct PlanStore {
    plans: Vec<Plan>,
    next_id: u64,
}

impl PlanStore {
    pub fn create(&mut self, ops: Vec<OpState>, note: &str, origin: &str) -> &Plan {
        self.next_id += 1;
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.plans.push(Plan {
            id: self.next_id,
            note: note.into(),
            origin: origin.into(),
            created,
            status: Status::Pending,
            ops,
        });
        self.plans.last().unwrap()
    }

    pub fn get(&self, id: u64) -> Option<&Plan> {
        self.plans.iter().find(|p| p.id == id)
    }

    pub fn list(&self) -> &[Plan] {
        &self.plans
    }

    fn get_mut(&mut self, id: u64) -> Result<&mut Plan, String> {
        self.plans
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("no plan {id}"))
    }

    /// Pending and valid → approved. Returns the ops for the caller to run.
    pub fn approve(&mut self, id: u64) -> Result<Vec<Op>, String> {
        let p = self.get_mut(id)?;
        if p.status != Status::Pending {
            return Err(format!("plan {id} is {}, not pending", p.status.as_str()));
        }
        if !p.valid() {
            return Err(format!(
                "plan {id} has ops with errors; propose a new plan without them"
            ));
        }
        p.status = Status::Approved;
        Ok(p.ops.iter().map(|o| o.op.clone()).collect())
    }

    /// Records per-op results of an approved plan → done (all ok) or failed.
    pub fn finish(&mut self, id: u64, results: Vec<Result<String, String>>) -> Result<(), String> {
        let p = self.get_mut(id)?;
        if p.status != Status::Approved {
            return Err(format!("plan {id} is {}, not approved", p.status.as_str()));
        }
        let all_ok = results.iter().all(|r| r.is_ok());
        for (o, r) in p.ops.iter_mut().zip(results) {
            o.result = Some(r);
        }
        p.status = if all_ok { Status::Done } else { Status::Failed };
        Ok(())
    }

    pub fn reject(&mut self, id: u64) -> Result<(), String> {
        let p = self.get_mut(id)?;
        if p.status != Status::Pending {
            return Err(format!("plan {id} is {}, not pending", p.status.as_str()));
        }
        p.status = Status::Rejected;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op_state(path: &str, errors: Vec<String>) -> OpState {
        OpState {
            op: Op {
                path: path.into(),
                action: Action::Trash,
            },
            size: 100,
            dest: None,
            errors,
            warnings: vec![],
            result: None,
        }
    }

    #[test]
    fn parses_ops() {
        let ops = parse_ops(&json!([
            {"action": "trash", "path": "/a"},
            {"action": "move", "path": "/b", "volume": "/Volumes/X", "link": true}
        ]))
        .unwrap();
        assert_eq!(ops[0].action, Action::Trash);
        assert_eq!(
            ops[1].action,
            Action::Move {
                volume: "/Volumes/X".into(),
                link: true
            }
        );
        assert!(parse_ops(&json!([{"action": "delete", "path": "/a"}])).is_err());
        assert!(parse_ops(&json!([{"action": "move", "path": "/a"}])).is_err());
        assert!(parse_ops(&json!([])).is_err());
    }

    #[test]
    fn pending_approved_done() {
        let mut s = PlanStore::default();
        let id = s
            .create(
                vec![op_state("/a", vec![]), op_state("/b", vec![])],
                "n",
                "web",
            )
            .id;
        assert_eq!(s.get(id).unwrap().total_bytes(), 200);
        assert_eq!(s.approve(id).unwrap().len(), 2);
        assert_eq!(s.get(id).unwrap().status, Status::Approved);
        assert!(s.approve(id).is_err(), "cannot approve twice");
        assert!(s.reject(id).is_err(), "cannot reject a running plan");
        s.finish(id, vec![Ok("ok".into()), Ok("ok".into())])
            .unwrap();
        assert_eq!(s.get(id).unwrap().status, Status::Done);
        assert!(s.finish(id, vec![]).is_err());
    }

    #[test]
    fn any_failed_op_fails_the_plan() {
        let mut s = PlanStore::default();
        let id = s
            .create(
                vec![op_state("/a", vec![]), op_state("/b", vec![])],
                "",
                "mcp",
            )
            .id;
        s.approve(id).unwrap();
        s.finish(id, vec![Ok("ok".into()), Err("boom".into())])
            .unwrap();
        let p = s.get(id).unwrap();
        assert_eq!(p.status, Status::Failed);
        assert_eq!(p.to_json()["ops"][1]["result"]["message"], "boom");
    }

    #[test]
    fn rejected_and_invalid_plans_never_run() {
        let mut s = PlanStore::default();
        let id = s.create(vec![op_state("/a", vec![])], "", "web").id;
        s.reject(id).unwrap();
        assert_eq!(s.get(id).unwrap().status, Status::Rejected);
        assert!(s.approve(id).is_err());

        let bad = s
            .create(
                vec![op_state("/System", vec!["protected".into()])],
                "",
                "mcp",
            )
            .id;
        assert!(s.approve(bad).is_err());
        assert_eq!(s.get(bad).unwrap().status, Status::Pending);
        assert!(s.approve(999).is_err());
    }

    #[test]
    fn validate_trash_refuses_protected_and_missing() {
        let st = validate(&Op {
            path: "/System".into(),
            action: Action::Trash,
        });
        assert!(!st.errors.is_empty());
        let st = validate(&Op {
            path: "/nonexistent-mds-path/x".into(),
            action: Action::Trash,
        });
        assert!(!st.errors.is_empty());
    }
}
