use std::collections::BTreeMap;

use serde::Serialize;

// JSON-клиент не должен округлять идентификаторы, превышающие 2^53.
fn decimal_id<S: serde::Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PlanId(#[serde(serialize_with = "decimal_id")] pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct JobId(#[serde(serialize_with = "decimal_id")] pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Operation {
    Diagnose,
    TrialDns,
    UpgradeOfficial,
    UpgradeUserFlatpak,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Authority {
    User,
    AuthorizedHelper,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum OnFailure {
    Stop,
    RestoreOwnedChanges,
    RequestRecovery,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlanStep {
    pub operation: Operation,
    pub reason: String,
    pub authority: Authority,
    pub reversible: bool,
    pub on_failure: OnFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Plan {
    pub id: PlanId,
    pub steps: Vec<PlanStep>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Outcome {
    Succeeded,
    Partial,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum JobPhase {
    AwaitingConsent,
    Ready,
    Running,
    Critical,
    Finished(Outcome),
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobEvent {
    Approve,
    Start,
    EnterCritical,
    Finish(Outcome),
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobError {
    EmptyPlan,
    PlanChanged,
    InvalidTransition,
    UnsafeToCancel,
}

impl JobPhase {
    /// Чистый переход. Ошибка не меняет фазу и не порождает эффектов.
    pub fn transition(self, event: JobEvent) -> Result<Self, JobError> {
        match (self, event) {
            (Self::AwaitingConsent, JobEvent::Approve) => Ok(Self::Ready),
            (Self::Ready, JobEvent::Start) => Ok(Self::Running),
            (Self::Running, JobEvent::EnterCritical) => Ok(Self::Critical),
            (Self::Running | Self::Critical, JobEvent::Finish(outcome)) => {
                Ok(Self::Finished(outcome))
            }
            (Self::AwaitingConsent | Self::Ready | Self::Running, JobEvent::Cancel) => {
                Ok(Self::Cancelled)
            }
            (Self::Critical, JobEvent::Cancel) => Err(JobError::UnsafeToCancel),
            _ => Err(JobError::InvalidTransition),
        }
    }
}

/// Модель регистрации задания, не исполнитель. До появления журнала живёт в памяти.
#[derive(Default)]
pub struct JobBook(BTreeMap<PlanId, (Plan, JobId)>);

impl JobBook {
    pub fn register(&mut self, plan: Plan) -> Result<JobId, JobError> {
        if plan.steps.is_empty() {
            return Err(JobError::EmptyPlan);
        }
        if let Some((original, job)) = self.0.get(&plan.id) {
            return if original == &plan {
                Ok(*job)
            } else {
                Err(JobError::PlanChanged)
            };
        }
        let job = JobId(self.0.len() as u64 + 1);
        self.0.insert(plan.id, (plan, job));
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_ids_do_not_lose_integer_precision() {
        let expected = serde_json::Value::String(u64::MAX.to_string());
        assert_eq!(serde_json::to_value(PlanId(u64::MAX)).unwrap(), expected);
        assert_eq!(serde_json::to_value(JobId(u64::MAX)).unwrap(), expected);
    }

    fn plan() -> Plan {
        Plan {
            id: PlanId(42),
            steps: vec![PlanStep {
                operation: Operation::Diagnose,
                reason: "Запрос пользователя".into(),
                authority: Authority::User,
                reversible: true,
                on_failure: OnFailure::Stop,
            }],
        }
    }

    #[test]
    fn repeated_request_attaches_but_changed_plan_needs_fresh_approval() {
        let mut book = JobBook::default();
        let id = book.register(plan()).unwrap();
        assert_eq!(book.register(plan()), Ok(id));
        let mut changed = plan();
        changed.steps[0].operation = Operation::UpgradeOfficial;
        assert_eq!(book.register(changed), Err(JobError::PlanChanged));
        assert_eq!(book.register(plan()), Ok(id));
        assert_eq!(
            book.register(Plan {
                id: PlanId(1),
                steps: vec![]
            }),
            Err(JobError::EmptyPlan)
        );
    }

    #[test]
    fn consent_and_irreversible_boundary_cannot_be_skipped() {
        use JobEvent::*;
        let pending = JobPhase::AwaitingConsent;
        assert_eq!(pending.transition(Start), Err(JobError::InvalidTransition));
        let running = pending
            .transition(Approve)
            .unwrap()
            .transition(Start)
            .unwrap();
        assert_eq!(running.transition(Cancel), Ok(JobPhase::Cancelled));
        let critical = running.transition(EnterCritical).unwrap();
        assert_eq!(critical.transition(Cancel), Err(JobError::UnsafeToCancel));
        let partial = critical.transition(Finish(Outcome::Partial)).unwrap();
        assert_eq!(partial, JobPhase::Finished(Outcome::Partial));
        assert_eq!(
            partial.transition(Finish(Outcome::Succeeded)),
            Err(JobError::InvalidTransition)
        );
    }
}
