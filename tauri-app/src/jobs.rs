//! Bounded, cancellable native jobs. Credentials are never part of job reports.
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::Emitter;

#[derive(Clone, Serialize, Deserialize)]
pub struct JobReport {
    pub id: String,
    pub kind: String,
    pub phase: String,
    pub message: String,
    pub completed: u64,
    pub total: u64,
    pub result: serde_json::Value,
}
pub struct Job {
    pub report: JobReport,
    pub cancel: Arc<AtomicBool>,
}
#[derive(Clone, Default)]
pub struct State(pub Arc<Mutex<Vec<Job>>>);
impl State {
    pub fn begin(&self, kind: &str) -> Result<(String, Arc<AtomicBool>), String> {
        let mut jobs = self.0.lock().map_err(|e| e.to_string())?;
        if jobs.iter().filter(|j| j.report.phase == "running").count() >= 8 {
            return Err("Eight jobs are already active.".into());
        }
        while jobs.len() >= 64 {
            let i = jobs
                .iter()
                .position(|j| j.report.phase != "running")
                .ok_or("Job queue full")?;
            jobs.remove(i);
        }
        let id = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        );
        let cancel = Arc::new(AtomicBool::new(false));
        jobs.push(Job {
            report: JobReport {
                id: id.clone(),
                kind: kind.into(),
                phase: "running".into(),
                message: String::new(),
                completed: 0,
                total: 0,
                result: serde_json::json!({}),
            },
            cancel: cancel.clone(),
        });
        Ok((id, cancel))
    }
    pub fn update(&self, app: &tauri::AppHandle, id: &str, update: impl FnOnce(&mut JobReport)) {
        if let Ok(mut jobs) = self.0.lock() {
            if let Some(job) = jobs.iter_mut().find(|j| j.report.id == id) {
                update(&mut job.report);
                let _ = app.emit("launcher-job", &job.report);
            }
        }
    }
    pub fn finish(
        &self,
        app: &tauri::AppHandle,
        id: &str,
        result: Result<serde_json::Value, String>,
    ) {
        let cancelled = self
            .0
            .lock()
            .ok()
            .and_then(|j| {
                j.iter()
                    .find(|j| j.report.id == id)
                    .map(|j| j.cancel.load(Ordering::Relaxed))
            })
            .unwrap_or(true);
        self.update(app, id, |r| match result {
            Ok(mut value) => {
                if let Some(selection) = r.result.get("selection").cloned() {
                    value["selection"] = selection;
                }
                r.phase = "complete".into();
                r.result = value;
            }
            Err(error) => {
                r.phase = if cancelled
                    && (error.starts_with("Cancelled")
                        || error.starts_with("Paused")
                        || error.contains("cancelled"))
                {
                    "cancelled"
                } else {
                    "failed"
                }
                .into();
                r.message = error;
            }
        });
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let jobs = self.0.lock().map_err(|e| e.to_string())?;
        let job = jobs
            .iter()
            .find(|j| j.report.id == id)
            .ok_or("Job no longer exists")?;
        job.cancel.store(true, Ordering::Relaxed);
        Ok(())
    }
    pub fn shutdown(&self) {
        if let Ok(jobs) = self.0.lock() {
            for j in jobs.iter() {
                j.cancel.store(true, Ordering::Relaxed);
            }
        }
    }
}
pub fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Cancelled".into())
    } else {
        Ok(())
    }
}
